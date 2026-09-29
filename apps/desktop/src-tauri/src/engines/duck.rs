use std::path::Path;
use std::sync::{Arc, Mutex};

use duckdb::arrow::array::{Array, RecordBatch};
use duckdb::arrow::datatypes::DataType;
use duckdb::arrow::util::display::{ArrayFormatter, FormatOptions};
use duckdb::{AccessMode, Config, Connection, InterruptHandle};

use crate::engines::db::{held, QueryResult, SessionConfig, TableInfo};

pub struct Duck {
    path: String,
    connection: Arc<Mutex<Result<Connection, String>>>,
    interrupt: Arc<Mutex<Arc<InterruptHandle>>>,
}

fn text(error: duckdb::Error) -> String {
    error.to_string()
}

// duckdb has no in-memory database to open read only, so that one stays
// writable on the server and GPQL alone refuses its writes
fn connect(path: &str, read_only: bool) -> Result<Connection, String> {
    if path.is_empty() {
        return Connection::open_in_memory().map_err(text);
    }

    let mode = if read_only {
        AccessMode::ReadOnly
    } else {
        AccessMode::Automatic
    };
    let config = Config::default().access_mode(mode).map_err(text)?;

    Connection::open_with_flags(path, config).map_err(text)
}

// the old handle has to close first, or duckdb trips over its own file lock
fn swap(slot: &mut Result<Connection, String>, path: &str, read_only: bool) -> Result<(), String> {
    drop(std::mem::replace(slot, Err(String::new())));

    match connect(path, read_only) {
        Ok(fresh) => {
            *slot = Ok(fresh);

            Ok(())
        }
        Err(failure) => {
            *slot = connect(path, !read_only)
                .map_err(|again| format!("the database could not be opened again: {again}"));

            Err(failure)
        }
    }
}

impl Duck {
    pub async fn open(config: &SessionConfig) -> Result<Self, String> {
        let path = config.path.clone();
        let read_only = config.read_only;
        let create = config.create;

        let opened = path.clone();
        let connection = tokio::task::spawn_blocking(move || {
            if !opened.is_empty() && !create && !Path::new(&opened).exists() {
                return Err(format!("{opened} does not exist"));
            }

            connect(&opened, read_only)
        })
        .await
        .map_err(|error| error.to_string())??;

        Ok(Duck {
            path,
            interrupt: Arc::new(Mutex::new(connection.interrupt_handle())),
            connection: Arc::new(Mutex::new(Ok(connection))),
        })
    }

    // a running statement holds the connection lock, so the handle lives outside it
    pub fn cancel(&self) {
        held(&self.interrupt).interrupt();
    }

    pub fn in_memory(&self) -> bool {
        self.path.is_empty()
    }

    async fn with<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        let shared = Arc::clone(&self.connection);

        tokio::task::spawn_blocking(move || match &mut *held(&shared) {
            Ok(connection) => work(connection),
            Err(broken) => Err(broken.clone()),
        })
        .await
        .map_err(|error| error.to_string())?
    }

    pub async fn query(&self, sql: &str) -> Result<QueryResult, String> {
        let sql = sql.to_string();

        self.with(move |connection| {
            let mut statement = connection.prepare(&sql).map_err(text)?;
            let answer = statement.query_arrow([]).map_err(text)?;
            let schema = answer.get_schema();
            let batches: Vec<RecordBatch> = answer.collect();

            let columns = schema
                .fields()
                .iter()
                .map(|field| field.name().clone())
                .collect();

            Ok(QueryResult {
                columns,
                rows: arrow_rows(&batches, Stamps::Local)?,
                affected: None,
            })
        })
        .await
    }

    pub async fn execute(&self, sql: &str) -> Result<(), String> {
        let sql = sql.to_string();

        self.with(move |connection| connection.execute_batch(&sql).map_err(text))
            .await
    }

    // parse and bind errors leave the transaction usable, runtime errors abort it
    pub async fn aborted(&self) -> bool {
        self.execute("select 1")
            .await
            .is_err_and(|failure| failure.contains("transaction is aborted"))
    }

    pub async fn reopen(&self, read_only: bool) -> Result<(), String> {
        if self.in_memory() {
            return Ok(());
        }

        let path = self.path.clone();
        let shared = Arc::clone(&self.connection);
        let interrupt = Arc::clone(&self.interrupt);

        tokio::task::spawn_blocking(move || {
            let mut slot = held(&shared);
            let outcome = swap(&mut slot, &path, read_only);

            if let Ok(connection) = slot.as_ref() {
                *held(&interrupt) = connection.interrupt_handle();
            }

            outcome
        })
        .await
        .map_err(|error| error.to_string())?
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let listing = self
            .query(
                "select table_name, estimated_size
                 from duckdb_tables()
                 order by table_name",
            )
            .await?;

        Ok(listing
            .rows
            .into_iter()
            .map(|row| TableInfo {
                name: row.first().cloned().flatten().unwrap_or_default(),
                rows: row
                    .get(1)
                    .cloned()
                    .flatten()
                    .and_then(|count| count.parse().ok())
                    .unwrap_or(0),
            })
            .collect())
    }

    pub async fn columns(&self) -> Result<QueryResult, String> {
        self.query(
            "select c.table_name, c.column_name, c.data_type,
                    case when c.is_nullable then 'YES' else 'NO' end,
                    case when exists (
                      select 1 from duckdb_constraints() k
                      where k.constraint_type = 'PRIMARY KEY'
                        and k.schema_name = c.schema_name
                        and k.table_name = c.table_name
                        and list_contains(k.constraint_column_names, c.column_name)
                    ) then 'PRI' else '' end,
                    '', ''
             from duckdb_columns() c
             order by c.table_name, c.column_index",
        )
        .await
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Stamps {
    Local,
    Utc,
}

// arrow already knows how to print every type duckdb and influx hand back,
// dates, decimals, intervals and nested lists included
pub(crate) fn arrow_rows(
    batches: &[RecordBatch],
    stamps: Stamps,
) -> Result<Vec<Vec<Option<String>>>, String> {
    let options = match stamps {
        Stamps::Local => FormatOptions::new()
            .with_timestamp_format(Some("%Y-%m-%d %H:%M:%S%.f"))
            .with_timestamp_tz_format(Some("%Y-%m-%d %H:%M:%S%.f%:z")),
        Stamps::Utc => FormatOptions::new()
            .with_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.fZ"))
            .with_timestamp_tz_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z")),
    };

    let mut rows = Vec::new();

    for batch in batches {
        let printers = batch
            .columns()
            .iter()
            .map(|array| {
                let binary = matches!(
                    array.data_type(),
                    DataType::Binary
                        | DataType::LargeBinary
                        | DataType::BinaryView
                        | DataType::FixedSizeBinary(_)
                );

                ArrayFormatter::try_new(array.as_ref(), &options)
                    .map(|printer| (array, printer, binary))
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;

        for index in 0..batch.num_rows() {
            rows.push(
                printers
                    .iter()
                    .map(|(array, printer, binary)| {
                        if array.is_null(index) {
                            return None;
                        }

                        let shown = printer.value(index).to_string();

                        Some(if *binary { format!("0x{shown}") } else { shown })
                    })
                    .collect(),
            );
        }
    }

    Ok(rows)
}
