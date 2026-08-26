use std::sync::Mutex;

use duckdb::Connection;
use duckdb::types::ValueRef;

use crate::engines::db::{QueryResult, SessionConfig, TableInfo};

pub struct Duck {
    connection: Mutex<Connection>,
}

impl Duck {
    pub fn open(config: &SessionConfig) -> Result<Self, String> {
        let connection = if config.path.is_empty() {
            Connection::open_in_memory()
        } else {
            Connection::open(&config.path)
        }
        .map_err(|error| error.to_string())?;

        return Ok(Duck {
            connection: Mutex::new(connection),
        });
    }

    pub fn query(&self, sql: &str) -> Result<QueryResult, String> {
        let connection = self.connection.lock().unwrap();
        let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
        let mut cursor = statement.query([]).map_err(|error| error.to_string())?;

        let mut columns: Vec<String> = Vec::new();
        let mut rows = Vec::new();

        while let Some(row) = cursor.next().map_err(|error| error.to_string())? {
            if columns.is_empty() {
                columns = row
                    .as_ref()
                    .column_names()
                    .iter()
                    .map(|name| name.to_string())
                    .collect();
            }

            let mut values = Vec::with_capacity(columns.len());

            for index in 0..columns.len() {
                let cell = row.get_ref(index).map_err(|error| error.to_string())?;

                values.push(text_of(cell));
            }

            rows.push(values);
        }

        if columns.is_empty() {
            columns = statement
                .column_names()
                .iter()
                .map(|name| name.to_string())
                .collect();
        }

        return Ok(QueryResult {
            columns,
            rows,
            affected: None,
        });
    }

    pub fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let listing = self.query(
            "select table_name, estimated_size
             from duckdb_tables()
             order by table_name",
        )?;

        return Ok(listing
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

    pub fn columns(&self) -> Result<QueryResult, String> {
        return self.query(
            "select table_name, column_name, data_type,
                    case when is_nullable then 'YES' else 'NO' end,
                    '', '', ''
             from duckdb_columns()
             order by table_name, column_index",
        );
    }
}

fn text_of(value: ValueRef<'_>) -> Option<String> {
    return match value {
        ValueRef::Null => None,
        ValueRef::Boolean(flag) => Some(flag.to_string()),
        ValueRef::TinyInt(number) => Some(number.to_string()),
        ValueRef::SmallInt(number) => Some(number.to_string()),
        ValueRef::Int(number) => Some(number.to_string()),
        ValueRef::BigInt(number) => Some(number.to_string()),
        ValueRef::HugeInt(number) => Some(number.to_string()),
        ValueRef::UTinyInt(number) => Some(number.to_string()),
        ValueRef::USmallInt(number) => Some(number.to_string()),
        ValueRef::UInt(number) => Some(number.to_string()),
        ValueRef::UBigInt(number) => Some(number.to_string()),
        ValueRef::Float(number) => Some(number.to_string()),
        ValueRef::Double(number) => Some(number.to_string()),
        ValueRef::Text(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => Some(format!("{} bytes", bytes.len())),
        other => Some(format!("{other:?}")),
    };
}
