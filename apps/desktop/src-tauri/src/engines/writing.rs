use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::atomic::Ordering;

use serde::Deserialize;
use tokio_postgres::types::private::BytesMut;
use tokio_postgres::types::{to_sql_checked, Format, IsNull, ToSql, Type};

use super::db::{
    held, literal, quote_for, quote_ident, read, reopen_sqlite, with_sqlite, Engine, Session,
};
use super::errors::{friendly, friendly_pg};
use super::slicing::binary_columns;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub keys: HashMap<String, Option<String>>,
    pub set: HashMap<String, Option<String>>,
}

struct Bound {
    sql: String,
    values: Vec<Option<String>>,
}

struct Builder<'a> {
    session: &'a Session,
    bind: bool,
    binary: &'a HashSet<String>,
    values: Vec<Option<String>>,
}

impl Builder<'_> {
    fn slot(&mut self, value: &Option<String>) -> String {
        if !self.bind || value.is_none() {
            return literal(&self.session.flavour, value);
        }

        self.values.push(value.clone());

        match self.session.engine {
            Engine::Postgres(_) => format!("${}", self.values.len()),
            _ => "?".to_string(),
        }
    }

    // the grid shows a binary value as 0x hex, so that is what comes back as
    // its key or its new value
    fn value(&mut self, column: &str, value: &Option<String>) -> String {
        let digits = value
            .as_deref()
            .and_then(|text| text.strip_prefix("0x"))
            .filter(|digits| {
                digits.len() % 2 == 0 && digits.chars().all(|c| c.is_ascii_hexdigit())
            });

        match digits {
            Some(digits) if self.binary.contains(column) => {
                format!("unhex({})", self.slot(&Some(digits.to_string())))
            }
            _ => self.slot(value),
        }
    }

    fn update(mut self, table: &str, edit: &Edit) -> Bound {
        let mut set: Vec<_> = edit.set.iter().collect();
        let mut keys: Vec<_> = edit.keys.iter().collect();

        set.sort();
        keys.sort();

        let assignments = set
            .into_iter()
            .map(|(column, value)| {
                format!(
                    "{} = {}",
                    quote_for(self.session, column),
                    self.value(column, value)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let conditions = keys
            .into_iter()
            .map(|(column, value)| match value {
                None => format!("{} is null", quote_for(self.session, column)),
                Some(_) => {
                    format!(
                        "{} = {}",
                        quote_for(self.session, column),
                        self.value(column, value)
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(" and ");

        Bound {
            sql: format!(
                "update {} set {assignments} where {conditions}",
                quote_for(self.session, table)
            ),
            values: self.values,
        }
    }
}

fn binds(session: &Session) -> bool {
    match &session.engine {
        Engine::Postgres(_) | Engine::MySql(_) | Engine::Sqlite(_) => true,
        Engine::Driver(driver) => driver.binds(),
        _ => false,
    }
}

async fn build(
    session: &Session,
    table: &str,
    edits: &[Edit],
    bind: bool,
) -> Result<Vec<Bound>, String> {
    let binary = binary_columns(session, table).await.unwrap_or_default();

    Ok(edits
        .iter()
        .filter(|edit| !edit.keys.is_empty() && !edit.set.is_empty())
        .map(|edit| {
            Builder {
                session,
                bind,
                binary: &binary,
                values: Vec::new(),
            }
            .update(table, edit)
        })
        .collect())
}

// the preview and the write share this builder, so what the user approves is
// exactly what runs, with each bound value written out in place
pub async fn edit_statements(
    session: &Session,
    table: &str,
    edits: &[Edit],
) -> Result<Vec<String>, String> {
    Ok(build(session, table, edits, false)
        .await?
        .into_iter()
        .map(|bound| bound.sql)
        .collect())
}

#[derive(Debug)]
struct Text(Option<String>);

// sent in text format the server parses the value by the column's own type,
// exactly as it would a quoted literal, so no rust type has to match it
impl ToSql for Text {
    fn to_sql(
        &self,
        _ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        match &self.0 {
            None => Ok(IsNull::Yes),
            Some(text) => {
                out.extend_from_slice(text.as_bytes());

                Ok(IsNull::No)
            }
        }
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }

    fn encode_format(&self, _ty: &Type) -> Format {
        Format::Text
    }

    to_sql_checked!();
}

async fn execute(session: &Session, bound: &Bound) -> Result<u64, String> {
    let done = match &session.engine {
        Engine::Postgres(client) => {
            let values: Vec<Text> = bound.values.iter().cloned().map(Text).collect();
            let params: Vec<&(dyn ToSql + Sync)> = values
                .iter()
                .map(|value| value as &(dyn ToSql + Sync))
                .collect();

            client
                .execute(&bound.sql, &params)
                .await
                .map_err(friendly_pg)
        }
        Engine::MySql(client) => client.execute(&bound.sql, &bound.values).await,
        Engine::Sqlite(connection) => {
            let sql = bound.sql.clone();
            let values = bound.values.clone();

            with_sqlite(connection, move |connection| {
                connection
                    .execute(&sql, rusqlite::params_from_iter(values.iter()))
                    .map(|count| count as u64)
                    .map_err(friendly)
            })
            .await
        }
        Engine::Driver(driver) if driver.binds() => driver.execute(&bound.sql, &bound.values).await,
        // duckdb reports an update as a one-cell Count row, not an affected count
        _ => {
            return read(session, &bound.sql).await.map(|result| {
                let counted = match result.rows.as_slice() {
                    [row] if row.len() == 1 => {
                        row[0].as_deref().and_then(|count| count.parse().ok())
                    }
                    _ => None,
                };

                result.affected.or(counted).unwrap_or(0)
            });
        }
    };

    if let Err(failure) = &done {
        note_failure(session, failure).await;
    }

    done
}

async fn execute_all(session: &Session, statements: &[Bound]) -> Result<u64, String> {
    let mut touched = 0;

    for bound in statements {
        touched += execute(session, bound).await?;
    }

    Ok(touched)
}

pub async fn apply(session: &Session, table: &str, edits: &[Edit]) -> Result<u64, String> {
    if session.read_only.load(Ordering::Relaxed) {
        return Err("this session is read only".into());
    }

    let statements = build(session, table, edits, binds(session)).await?;
    held(&session.order_keys).clear();

    if session.manual.load(Ordering::Relaxed) {
        begin_if_manual(session).await?;

        return execute_all(session, &statements).await;
    }

    if statements.len() < 2 || !transactional(session) {
        return execute_all(session, &statements).await;
    }

    let _guard = session.tx_lock.lock().await;

    control(session, "begin").await?;

    match execute_all(session, &statements).await {
        Ok(touched) => {
            control(session, "commit").await?;

            Ok(touched)
        }
        Err(failure) => {
            let _ = control(session, "rollback").await;

            Err(failure)
        }
    }
}

// greptime answers begin and commit without keeping a transaction
pub fn transactional(session: &Session) -> bool {
    match session.engine {
        Engine::Postgres(_) => session.flavour != "greptimedb",
        Engine::MySql(_) | Engine::Sqlite(_) | Engine::Duck(_) => true,
        _ => false,
    }
}

async fn control(session: &Session, word: &str) -> Result<(), String> {
    match &session.engine {
        Engine::Postgres(client) => client.batch_execute(word).await.map_err(friendly_pg),
        Engine::Sqlite(connection) => {
            let word = word.to_string();

            with_sqlite(connection, move |connection| {
                connection.execute_batch(&word).map_err(friendly)
            })
            .await
        }
        Engine::MySql(client) => client.query(word).await.map(|_| ()),
        Engine::Duck(duck) => duck.execute(word).await,
        _ => Err("this engine has no transactions".into()),
    }
}

// the later commit has to report a transaction the server already gave up on
pub(crate) async fn note_failure(session: &Session, failure: &str) {
    if !session.open_tx.load(Ordering::Relaxed) {
        return;
    }

    let lost = match &session.engine {
        Engine::Postgres(_) => true,
        Engine::Duck(duck) => duck.aborted().await,
        Engine::MySql(_) => failure.contains("Deadlock found"),
        _ => false,
    };

    let mut slot = held(&session.tx_failure);

    if lost && slot.is_none() {
        *slot = Some(failure.to_string());
    }
}

// a duckdb commit on an aborted transaction answers success and keeps nothing
async fn server_in_tx(session: &Session) -> Option<bool> {
    match &session.engine {
        Engine::Sqlite(connection) => {
            with_sqlite(connection, |connection| Ok(!connection.is_autocommit()))
                .await
                .ok()
        }
        Engine::Duck(duck) => Some(!duck.aborted().await),
        _ => None,
    }
}

fn forget(session: &Session) {
    session.open_tx.store(false, Ordering::Relaxed);
    *held(&session.tx_failure) = None;
}

// set search_path is transactional, so a rollback would quietly put the
// schema the ui still shows back to the old one
async fn restore_search_path(session: &Session) {
    let Engine::Postgres(client) = &session.engine else {
        return;
    };

    let Some(path) = held(&session.search_path).clone() else {
        return;
    };

    let _ = client
        .batch_execute(&format!("set search_path to {}", quote_ident(&path)))
        .await;
}

// a manual-commit session holds one transaction open from the first write
// until the user commits or rolls back
pub async fn begin_if_manual(session: &Session) -> Result<(), String> {
    if !session.manual.load(Ordering::Relaxed) || !transactional(session) {
        return Ok(());
    }

    let _guard = session.tx_lock.lock().await;

    if session.open_tx.load(Ordering::Relaxed) {
        return Ok(());
    }

    control(session, "begin").await?;
    session.open_tx.store(true, Ordering::Relaxed);

    Ok(())
}

pub async fn set_manual(session: &Session, on: bool) -> Result<(), String> {
    if on && !transactional(session) {
        return Err("this engine has no transactions".into());
    }

    if !on {
        finish(session, "rollback").await?;
    }

    session.manual.store(on, Ordering::Relaxed);

    Ok(())
}

pub async fn finish(session: &Session, word: &str) -> Result<bool, String> {
    let _guard = session.tx_lock.lock().await;

    if !session.open_tx.load(Ordering::Relaxed) {
        return Ok(false);
    }

    if !word.eq_ignore_ascii_case("commit") {
        let rolled = control(session, "rollback").await;

        forget(session);
        restore_search_path(session).await;
        rolled?;

        return Ok(true);
    }

    let mut broken = held(&session.tx_failure).clone();

    if broken.is_none() && server_in_tx(session).await == Some(false) {
        broken = Some("the database had already rolled the transaction back".to_string());
    }

    if let Some(failure) = broken {
        let _ = control(session, "rollback").await;

        forget(session);
        restore_search_path(session).await;

        return Err(format!(
            "nothing was committed, because a statement in this transaction failed: {failure}"
        ));
    }

    if let Err(failure) = control(session, "commit").await {
        let still_open = match &session.engine {
            Engine::Postgres(_) | Engine::Duck(_) => false,
            _ => server_in_tx(session).await.unwrap_or(true),
        };

        if !still_open {
            forget(session);
            restore_search_path(session).await;
        }

        return Err(failure);
    }

    forget(session);

    Ok(true)
}

pub async fn set_read_only(session: &Session, on: bool) -> Result<(), String> {
    if on {
        session.set_read_only(true);
    }

    match switch_server(session, on).await {
        Ok(guarded) => {
            session.server_guard.store(on && guarded, Ordering::Relaxed);
            session.set_read_only(on);

            Ok(())
        }
        // GPQL alone keeps the session read only until a later switch succeeds
        Err(failure) => {
            if on {
                session.server_guard.store(false, Ordering::Relaxed);
            }

            Err(failure)
        }
    }
}

async fn switch_server(session: &Session, on: bool) -> Result<bool, String> {
    let open = session.open_tx.load(Ordering::Relaxed);
    let wish = if on { "read only" } else { "read write" };

    match &session.engine {
        Engine::Postgres(_) if session.flavour == "greptimedb" => Ok(false),
        Engine::Postgres(client) => {
            let mut sql = format!("set session characteristics as transaction {wish}");

            // the session default skips the open transaction, which may still turn read only
            if on && open {
                sql.push_str("; set transaction read only");
            }

            client.batch_execute(&sql).await.map_err(friendly_pg)?;

            Ok(true)
        }
        Engine::MySql(client) => {
            client
                .query(&format!("set session transaction {wish}"))
                .await?;

            Ok(!open)
        }
        Engine::Sqlite(_) => {
            if open {
                return Err(
                    "commit or roll back the open transaction before switching read only".into(),
                );
            }

            reopen_sqlite(session, on).await?;

            Ok(true)
        }
        Engine::Duck(duck) => {
            if open {
                return Err(
                    "commit or roll back the open transaction before switching read only".into(),
                );
            }

            duck.reopen(on).await?;

            Ok(true)
        }
        _ => Ok(true),
    }
}
