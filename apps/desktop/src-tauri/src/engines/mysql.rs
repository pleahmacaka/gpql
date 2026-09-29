use mysql_async::consts::ColumnType;
use mysql_async::prelude::Queryable;
use mysql_async::{Column, Conn, DriverError, Opts, OptsBuilder, Params, Row, SslOpts, Value};
use tokio::sync::Mutex;

use crate::engines::db::{hex, install_crypto, port_or, QueryResult, SessionConfig, TableInfo};

pub struct MySql {
    connection: Mutex<Conn>,
    id: u32,
    opts: Opts,
}

impl MySql {
    pub async fn open(config: &SessionConfig) -> Result<Self, String> {
        let host = if config.host.is_empty() {
            "127.0.0.1".to_string()
        } else {
            config.host.clone()
        };
        let dial = if config.dial.is_empty() {
            host.clone()
        } else {
            config.dial.clone()
        };

        let base = OptsBuilder::default()
            .ip_or_hostname(dial)
            .tcp_port(port_or(&config.port, 3306)?)
            .user(Some(config.user.clone()))
            .pass(Some(config.password.clone()))
            .db_name(Some(config.database.clone()));

        let tunnelled = !config.dial.is_empty();
        let secured = |verify: bool| {
            install_crypto();

            base.clone().ssl_opts(Some(
                SslOpts::default()
                    .with_danger_accept_invalid_certs(!verify)
                    .with_danger_skip_domain_validation(!verify)
                    .with_danger_tls_hostname_override(tunnelled.then(|| host.clone())),
            ))
        };

        // prefer only drops to plaintext when the server says it has no TLS,
        // which it does before any credential is sent
        let connected = match config.tls.as_str() {
            "disable" => Conn::new(base.clone()).await,
            "require" => Conn::new(secured(false)).await,
            "verify-full" => Conn::new(secured(true)).await,
            "" | "prefer" => match Conn::new(secured(false)).await {
                Err(mysql_async::Error::Driver(DriverError::NoClientSslFlagFromServer)) => {
                    Conn::new(base.clone()).await
                }
                other => other,
            },
            other => return Err(format!("{other} is not a TLS mode GPQL knows")),
        };

        let mut connection = connected.map_err(friendly)?;

        if config.read_only {
            connection
                .query_drop("set session transaction read only")
                .await
                .map_err(friendly)?;
        }

        Ok(MySql {
            id: connection.id(),
            opts: connection.opts().clone(),
            connection: Mutex::new(connection),
        })
    }

    // the session's own connection is busy with the query, so the kill has to
    // arrive over a second one
    pub async fn cancel(&self) -> Result<(), String> {
        let mut side = Conn::new(self.opts.clone()).await.map_err(friendly)?;

        side.query_drop(format!("kill query {}", self.id))
            .await
            .map_err(friendly)?;

        side.disconnect().await.map_err(friendly)
    }

    pub async fn query(&self, sql: &str) -> Result<QueryResult, String> {
        let mut connection = self.connection.lock().await;
        let mut answer = connection.query_iter(sql).await.map_err(friendly)?;

        let mut columns: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<Option<String>>> = Vec::new();

        while !answer.is_empty() {
            let described = answer.columns();
            let batch: Vec<Row> = answer.collect().await.map_err(friendly)?;

            let Some(described) = described.filter(|described| !described.is_empty()) else {
                continue;
            };

            let binary: Vec<bool> = described.iter().map(binary).collect();

            columns = described
                .iter()
                .map(|column| column.name_str().to_string())
                .collect();
            rows = batch
                .into_iter()
                .map(|row| {
                    (0..row.len())
                        .map(|index| {
                            row.as_ref(index)
                                .and_then(|value| text_of(value, binary[index]))
                        })
                        .collect()
                })
                .collect();
        }

        let affected = answer.affected_rows();

        Ok(QueryResult {
            columns,
            rows,
            affected: Some(affected),
        })
    }

    pub async fn execute(&self, sql: &str, values: &[Option<String>]) -> Result<u64, String> {
        let mut connection = self.connection.lock().await;
        let params: Vec<Value> = values
            .iter()
            .map(|value| match value {
                None => Value::NULL,
                Some(text) => Value::Bytes(text.as_bytes().to_vec()),
            })
            .collect();

        connection
            .exec_drop(sql, Params::Positional(params))
            .await
            .map_err(friendly)?;

        Ok(connection.affected_rows())
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let result = self
            .query(
                "select table_name, coalesce(table_rows, 0)
                 from information_schema.tables
                 where table_schema = database()
                 order by table_name",
            )
            .await?;

        Ok(result
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
                        c.is_nullable, c.column_key,
                        case when k.referenced_table_schema = c.table_schema
                             then k.referenced_table_name
                             else concat(k.referenced_table_schema, '.',
                                         k.referenced_table_name) end,
                        k.referenced_column_name
                 from information_schema.columns c
                 left join information_schema.key_column_usage k
                   on k.table_schema = c.table_schema
                  and k.table_name = c.table_name
                  and k.column_name = c.column_name
                  and k.referenced_table_name is not null
                 where c.table_schema = database()
                 order by c.table_name, c.ordinal_position",
        )
        .await
    }
}

// numbers and dates also report the binary character set, so only the
// string and blob types count as raw bytes
fn binary(column: &Column) -> bool {
    let raw = matches!(
        column.column_type(),
        ColumnType::MYSQL_TYPE_STRING
            | ColumnType::MYSQL_TYPE_VAR_STRING
            | ColumnType::MYSQL_TYPE_VARCHAR
            | ColumnType::MYSQL_TYPE_TINY_BLOB
            | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
            | ColumnType::MYSQL_TYPE_LONG_BLOB
            | ColumnType::MYSQL_TYPE_BLOB
    );

    (raw && column.character_set() == 63)
        || matches!(
            column.column_type(),
            ColumnType::MYSQL_TYPE_GEOMETRY | ColumnType::MYSQL_TYPE_BIT
        )
}

fn text_of(value: &Value, binary: bool) -> Option<String> {
    match value {
        Value::NULL => None,
        Value::Bytes(bytes) if binary => Some(hex(bytes)),
        Value::Bytes(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Value::Int(number) => Some(number.to_string()),
        Value::UInt(number) => Some(number.to_string()),
        Value::Float(number) => Some(number.to_string()),
        Value::Double(number) => Some(number.to_string()),
        other => Some(format!("{other:?}")),
    }
}

fn friendly(error: mysql_async::Error) -> String {
    error.to_string()
}
