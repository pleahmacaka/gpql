use mysql_async::prelude::Queryable;
use mysql_async::{Conn, Opts, OptsBuilder, Row, Value};
use tokio::sync::Mutex;

use crate::db::{QueryResult, SessionConfig, TableInfo};

pub struct MySql {
    connection: Mutex<Conn>,
}

impl MySql {
    pub async fn open(config: &SessionConfig) -> Result<Self, String> {
        let port = config.port.parse::<u16>().unwrap_or(3306);
        let opts: Opts = OptsBuilder::default()
            .ip_or_hostname(if config.host.is_empty() {
                "127.0.0.1".to_string()
            } else {
                config.host.clone()
            })
            .tcp_port(port)
            .user(Some(config.user.clone()))
            .pass(Some(config.password.clone()))
            .db_name(Some(config.database.clone()))
            .into();

        let mut connection = Conn::new(opts).await.map_err(friendly)?;

        if config.read_only {
            connection
                .query_drop("set session transaction read only")
                .await
                .map_err(friendly)?;
        }

        return Ok(MySql {
            connection: Mutex::new(connection),
        });
    }

    pub async fn query(&self, sql: &str) -> Result<QueryResult, String> {
        let mut connection = self.connection.lock().await;
        let rows: Vec<Row> = connection.query(sql).await.map_err(friendly)?;

        let columns = rows
            .first()
            .map(|row| {
                row.columns_ref()
                    .iter()
                    .map(|column| column.name_str().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let shaped = rows
            .into_iter()
            .map(|row| {
                (0..row.len())
                    .map(|index| row.as_ref(index).map(text_of).unwrap_or(None))
                    .collect::<Vec<_>>()
            })
            .collect();

        return Ok(QueryResult {
            columns,
            rows: shaped,
            affected: Some(connection.affected_rows()),
        });
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

        return Ok(result
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
            .collect());
    }

    pub async fn columns(&self) -> Result<QueryResult, String> {
        return self
            .query(
                "select c.table_name, c.column_name, c.data_type,
                        c.is_nullable, c.column_key,
                        k.referenced_table_name, k.referenced_column_name
                 from information_schema.columns c
                 left join information_schema.key_column_usage k
                   on k.table_schema = c.table_schema
                  and k.table_name = c.table_name
                  and k.column_name = c.column_name
                  and k.referenced_table_name is not null
                 where c.table_schema = database()
                 order by c.table_name, c.ordinal_position",
            )
            .await;
    }
}

fn text_of(value: &Value) -> Option<String> {
    return match value {
        Value::NULL => None,
        Value::Bytes(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Value::Int(number) => Some(number.to_string()),
        Value::UInt(number) => Some(number.to_string()),
        Value::Float(number) => Some(number.to_string()),
        Value::Double(number) => Some(number.to_string()),
        other => Some(format!("{other:?}")),
    };
}

fn friendly(error: mysql_async::Error) -> String {
    let text = error.to_string();

    if text.contains("Access denied") {
        return "wrong user or password".into();
    }

    return text;
}
