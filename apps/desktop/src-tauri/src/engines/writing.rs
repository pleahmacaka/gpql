use std::collections::HashMap;
use std::sync::atomic::Ordering;

use serde::Deserialize;

use super::db::{literal, query, quote_for, Engine, Session};
use super::errors::{friendly, friendly_pg};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub keys: HashMap<String, Option<String>>,
    pub set: HashMap<String, Option<String>>,
}

// the preview and the write share this builder, so what the user approves is
// exactly what runs
pub fn edit_statements(session: &Session, table: &str, edits: &[Edit]) -> Vec<String> {
    let mut out = Vec::new();

    for edit in edits {
        if edit.keys.is_empty() || edit.set.is_empty() {
            continue;
        }

        let assignments = edit
            .set
            .iter()
            .map(|(column, value)| {
                format!("{} = {}", quote_for(session, column), literal(value))
            })
            .collect::<Vec<_>>()
            .join(", ");

        let conditions = edit
            .keys
            .iter()
            .map(|(column, value)| match value {
                None => format!("{} is null", quote_for(session, column)),
                Some(_) => {
                    format!("{} = {}", quote_for(session, column), literal(value))
                }
            })
            .collect::<Vec<_>>()
            .join(" and ");

        out.push(format!(
            "update {} set {assignments} where {conditions}",
            quote_for(session, table)
        ));
    }

    return out;
}

pub async fn apply(
    session: &Session,
    table: &str,
    edits: &[Edit],
) -> Result<u64, String> {
    if session.read_only.load(Ordering::Relaxed) {
        return Err("this session is read only".into());
    }

    let mut touched = 0;

    for sql in edit_statements(session, table, edits) {
        let result = query(session, &sql).await?;

        touched += result.affected.unwrap_or(0);
    }

    return Ok(touched);
}

pub fn transactional(session: &Session) -> bool {
    return matches!(
        session.engine,
        Engine::Postgres(_) | Engine::MySql(_) | Engine::Sqlite(_) | Engine::Duck(_)
    );
}

async fn control(session: &Session, word: &str) -> Result<(), String> {
    match &session.engine {
        Engine::Postgres(client) => {
            client.batch_execute(word).await.map_err(friendly_pg)?;
        }
        Engine::Sqlite(connection) => {
            connection
                .lock()
                .unwrap()
                .execute_batch(word)
                .map_err(friendly)?;
        }
        Engine::MySql(client) => {
            client.query(word).await?;
        }
        Engine::Duck(duck) => {
            duck.query(word)?;
        }
        _ => return Err("this engine has no transactions".into()),
    }

    return Ok(());
}

// a manual-commit session holds one transaction open from the first write
// until the user commits or rolls back
pub async fn begin_if_manual(session: &Session) -> Result<(), String> {
    if !session.manual.load(Ordering::Relaxed) || !transactional(session) {
        return Ok(());
    }

    if session.open_tx.swap(true, Ordering::Relaxed) {
        return Ok(());
    }

    if let Err(failure) = control(session, "begin").await {
        session.open_tx.store(false, Ordering::Relaxed);

        return Err(failure);
    }

    return Ok(());
}

pub async fn set_manual(session: &Session, on: bool) -> Result<(), String> {
    if on && !transactional(session) {
        return Err("this engine has no transactions".into());
    }

    if !on {
        finish(session, "rollback").await?;
    }

    session.manual.store(on, Ordering::Relaxed);

    return Ok(());
}

pub async fn finish(session: &Session, word: &str) -> Result<bool, String> {
    if !session.open_tx.swap(false, Ordering::Relaxed) {
        return Ok(false);
    }

    control(session, word).await?;

    return Ok(true);
}

pub async fn set_read_only(session: &Session, on: bool) -> Result<(), String> {
    session.set_read_only(on);

    if let Engine::Postgres(client) = &session.engine {
        let wish = if on { "read only" } else { "read write" };

        client
            .batch_execute(&format!(
                "set session characteristics as transaction {wish}"
            ))
            .await
            .map_err(friendly_pg)?;
    }

    return Ok(());
}
