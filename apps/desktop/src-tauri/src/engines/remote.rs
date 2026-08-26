use redis::aio::MultiplexedConnection;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::engines::db::{QueryResult, SessionConfig, TableInfo};

pub struct Http {
    pub flavour: String,
    pub url: String,
    pub token: String,
    pub database: String,
    client: reqwest::Client,
}

pub struct Graph {
    pub name: String,
    connection: Mutex<MultiplexedConnection>,
}

impl Http {
    pub fn open(config: &SessionConfig) -> Self {
        return Http {
            flavour: config.kind.clone(),
            url: config.url.trim_end_matches('/').to_string(),
            token: config.token.clone(),
            database: config.database.clone(),
            client: reqwest::Client::new(),
        };
    }

    pub async fn query(&self, sql: &str) -> Result<QueryResult, String> {
        return match self.flavour.as_str() {
            "supabase_api" => self.supabase(sql).await,
            _ => self.d1(sql).await,
        };
    }

    async fn d1(&self, sql: &str) -> Result<QueryResult, String> {
        let endpoint = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/d1/database/{}/query",
            self.url.trim_start_matches("https://"),
            self.database
        );

        let body: Value = self
            .client
            .post(endpoint)
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "sql": sql }))
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;

        if let Some(message) = body.pointer("/errors/0/message").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        let records = body
            .pointer("/result/0/results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        return Ok(records_to_grid(records));
    }

    async fn supabase(&self, sql: &str) -> Result<QueryResult, String> {
        let endpoint = format!(
            "https://api.supabase.com/v1/projects/{}/database/query",
            self.database
        );

        let body: Value = self
            .client
            .post(endpoint)
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "query": sql }))
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;

        if let Some(message) = body.get("message").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        let records = body.as_array().cloned().unwrap_or_default();

        return Ok(records_to_grid(records));
    }

    pub async fn columns(&self) -> Result<QueryResult, String> {
        let listing = self
            .query(
                "select c.table_name, c.column_name, c.data_type, c.is_nullable,
                        case when pk.column_name is null then '' else 'PRI' end
                          as column_key,
                        fk.target_table, fk.target_column
                 from information_schema.columns c
                 left join (
                   select kcu.table_name, kcu.column_name
                   from information_schema.table_constraints tc
                   join information_schema.key_column_usage kcu
                     on kcu.constraint_name = tc.constraint_name
                    and kcu.table_schema = tc.table_schema
                   where tc.constraint_type = 'PRIMARY KEY'
                     and tc.table_schema = 'public'
                 ) pk on pk.table_name = c.table_name
                     and pk.column_name = c.column_name
                 left join (
                   select kcu.table_name, kcu.column_name,
                          ccu.table_name as target_table,
                          ccu.column_name as target_column
                   from information_schema.table_constraints tc
                   join information_schema.key_column_usage kcu
                     on kcu.constraint_name = tc.constraint_name
                    and kcu.table_schema = tc.table_schema
                   join information_schema.constraint_column_usage ccu
                     on ccu.constraint_name = tc.constraint_name
                    and ccu.table_schema = tc.table_schema
                   where tc.constraint_type = 'FOREIGN KEY'
                     and tc.table_schema = 'public'
                 ) fk on fk.table_name = c.table_name
                     and fk.column_name = c.column_name
                 where c.table_schema = 'public'
                 order by c.table_name, c.ordinal_position",
            )
            .await?;

        return Ok(ordered(
            listing,
            &[
                "table_name",
                "column_name",
                "data_type",
                "is_nullable",
                "column_key",
                "target_table",
                "target_column",
            ],
        ));
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let listing = "select name from sqlite_master where type = 'table' \
             and name not like 'sqlite_%' order by name";

        let result = self.query(listing).await?;
        let column = result
            .columns
            .iter()
            .position(|name| {
                name == "table_name" || name == "name" || name == "label"
            })
            .unwrap_or(0);

        return Ok(result
            .rows
            .into_iter()
            .filter_map(|row| row.get(column).cloned().flatten())
            .map(|name| TableInfo { name, rows: 0 })
            .collect());
    }
}

impl Graph {
    pub async fn open(config: &SessionConfig) -> Result<Self, String> {
        let client = redis::Client::open(config.url.as_str())
            .map_err(|error| error.to_string())?;
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| error.to_string())?;

        return Ok(Graph {
            name: if config.database.is_empty() {
                "falkordb".into()
            } else {
                config.database.clone()
            },
            connection: Mutex::new(connection),
        });
    }

    pub async fn query(&self, cypher: &str) -> Result<QueryResult, String> {
        let mut connection = self.connection.lock().await;

        let answer: redis::Value = redis::cmd("GRAPH.QUERY")
            .arg(&self.name)
            .arg(cypher)
            .arg("--compact")
            .query_async(&mut *connection)
            .await
            .map_err(|error| error.to_string())?;

        return Ok(shape(answer));
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let result = self.query("call db.labels()").await?;

        return Ok(result
            .rows
            .into_iter()
            .filter_map(|row| row.into_iter().next().flatten())
            .map(|name| TableInfo { name, rows: 0 })
            .collect());
    }
}

fn shape(answer: redis::Value) -> QueryResult {
    let redis::Value::Array(parts) = answer else {
        return QueryResult {
            columns: vec!["result".into()],
            rows: vec![vec![flatten(&answer)]],
            affected: None,
        };
    };

    let columns = parts
        .first()
        .map(collect)
        .unwrap_or_default()
        .into_iter()
        .map(|cell| cell.unwrap_or_default())
        .collect::<Vec<_>>();

    let rows = match parts.get(1) {
        Some(redis::Value::Array(rows)) => rows.iter().map(collect).collect(),
        _ => Vec::new(),
    };

    return QueryResult { columns, rows, affected: None };
}

fn collect(value: &redis::Value) -> Vec<Option<String>> {
    return match value {
        redis::Value::Array(items) => items.iter().map(flatten).collect(),
        other => vec![flatten(other)],
    };
}

fn flatten(value: &redis::Value) -> Option<String> {
    return match value {
        redis::Value::Nil => None,
        redis::Value::Int(number) => Some(number.to_string()),
        redis::Value::Double(number) => Some(number.to_string()),
        redis::Value::Boolean(flag) => Some(flag.to_string()),
        redis::Value::BulkString(bytes) => {
            Some(String::from_utf8_lossy(bytes).into_owned())
        }
        redis::Value::SimpleString(text) => Some(text.clone()),
        redis::Value::Array(items) => Some(
            items
                .iter()
                .filter_map(flatten)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        other => Some(format!("{other:?}")),
    };
}

// json objects come back in whatever key order the server used; schema parsing
// reads by position, so pin the order before handing rows over.
fn ordered(listing: QueryResult, wanted: &[&str]) -> QueryResult {
    let spots: Vec<Option<usize>> = wanted
        .iter()
        .map(|name| listing.columns.iter().position(|held| held == name))
        .collect();

    let rows = listing
        .rows
        .into_iter()
        .map(|row| {
            spots
                .iter()
                .map(|spot| spot.and_then(|index| row.get(index).cloned().flatten()))
                .collect::<Vec<_>>()
        })
        .collect();

    return QueryResult {
        columns: wanted.iter().map(|name| name.to_string()).collect(),
        rows,
        affected: None,
    };
}

fn records_to_grid(records: Vec<Value>) -> QueryResult {
    let mut columns: Vec<String> = Vec::new();

    for record in &records {
        for key in record.as_object().map(|map| map.keys()).into_iter().flatten() {
            if !columns.contains(key) {
                columns.push(key.clone());
            }
        }
    }

    let rows = records
        .iter()
        .map(|record| {
            columns
                .iter()
                .map(|name| record.get(name).map(text_of).unwrap_or(None))
                .collect::<Vec<_>>()
        })
        .collect();

    return QueryResult { columns, rows, affected: None };
}



fn text_of(value: &Value) -> Option<String> {
    return match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    };
}
