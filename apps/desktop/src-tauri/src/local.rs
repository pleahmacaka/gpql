use std::sync::Mutex;

use rusqlite::types::{ToSqlOutput, Value, ValueRef};
use rusqlite::{Connection, params_from_iter};
use serde_json::{Number, Value as Json};

pub struct Local(pub Mutex<Connection>);

impl Local {
    pub fn open() -> Result<Self, String> {
        let folder = dirs::data_dir()
            .ok_or_else(|| "no app data folder on this machine".to_string())?
            .join("gpql");

        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;

        let connection = Connection::open(folder.join("gpql.db")).map_err(|e| e.to_string())?;
        connection
            .execute_batch("pragma journal_mode = wal; pragma foreign_keys = on;")
            .map_err(|e| e.to_string())?;

        return Ok(Local(Mutex::new(connection)));
    }
}

fn to_sql(value: &Json) -> ToSqlOutput<'static> {
    let mapped = match value {
        Json::Null => Value::Null,
        Json::Bool(flag) => Value::Integer(*flag as i64),
        Json::Number(number) => number
            .as_i64()
            .map(Value::Integer)
            .or_else(|| number.as_f64().map(Value::Real))
            .unwrap_or(Value::Null),
        Json::String(text) => Value::Text(text.clone()),
        other => Value::Text(other.to_string()),
    };

    return ToSqlOutput::Owned(mapped);
}

fn to_json(value: ValueRef<'_>) -> Json {
    match value {
        ValueRef::Null => Json::Null,
        ValueRef::Integer(number) => Json::Number(number.into()),
        ValueRef::Real(number) => Number::from_f64(number).map(Json::Number).unwrap_or(Json::Null),
        ValueRef::Text(bytes) => Json::String(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => Json::String(String::from_utf8_lossy(bytes).into_owned()),
    }
}

pub fn run(local: &Local, sql: &str, params: &[Json]) -> Result<Vec<Vec<Json>>, String> {
    let connection = local.0.lock().unwrap();
    let bound: Vec<ToSqlOutput<'static>> = params.iter().map(to_sql).collect();

    let mut statement = connection.prepare(sql).map_err(|e| e.to_string())?;
    let width = statement.column_count();

    if width == 0 {
        let affected = statement
            .execute(params_from_iter(bound))
            .map_err(|e| e.to_string())?;

        return Ok(vec![vec![Json::Number(Number::from(affected as u64))]]);
    }

    let mut rows = Vec::new();
    let mut cursor = statement
        .query(params_from_iter(bound))
        .map_err(|e| e.to_string())?;

    while let Some(row) = cursor.next().map_err(|e| e.to_string())? {
        let mut values = Vec::with_capacity(width);

        for index in 0..width {
            values.push(to_json(row.get_ref(index).map_err(|e| e.to_string())?));
        }

        rows.push(values);
    }

    return Ok(rows);
}

pub fn batch(local: &Local, sql: &str) -> Result<(), String> {
    let connection = local.0.lock().unwrap();

    return connection.execute_batch(sql).map_err(|e| e.to_string());
}
