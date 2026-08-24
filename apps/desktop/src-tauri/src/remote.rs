use redis::aio::MultiplexedConnection;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::db::{QueryResult, SessionConfig, TableInfo};

pub struct Http {
    pub flavour: String,
    pub url: String,
    pub token: String,
    pub database: String,
    pub user: String,
    pub password: String,
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
            user: config.user.clone(),
            password: config.password.clone(),
            client: reqwest::Client::new(),
        };
    }

    pub async fn query(&self, sql: &str) -> Result<QueryResult, String> {
        return match self.flavour.as_str() {
            "influxdb" => self.influx(sql).await,
            "clickhouse" => self.clickhouse(sql).await,
            "neo4j" => self.neo4j(sql).await,
            "snowflake" => self.snowflake(sql).await,
            "d1" => self.d1(sql).await,
            _ => self.turso(sql).await,
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

    async fn clickhouse(&self, sql: &str) -> Result<QueryResult, String> {
        let answer = self
            .client
            .post(&self.url)
            .query(&[("database", self.database.as_str())])
            .basic_auth(&self.user, Some(&self.password))
            .body(format!("{sql} format JSONCompact"))
            .send()
            .await
            .map_err(|error| error.to_string())?;

        if !answer.status().is_success() {
            return Err(answer
                .text()
                .await
                .unwrap_or_else(|_| "clickhouse refused that".into()));
        }

        let body: Value = answer.json().await.map_err(|error| error.to_string())?;

        return Ok(QueryResult {
            columns: named(body.get("meta"), "name"),
            rows: gridded(body.get("data")),
            affected: None,
        });
    }

    async fn neo4j(&self, cypher: &str) -> Result<QueryResult, String> {
        let database = if self.database.is_empty() {
            "neo4j"
        } else {
            &self.database
        };

        let body: Value = self
            .client
            .post(format!("{}/db/{database}/query/v2", self.url))
            .basic_auth(&self.user, Some(&self.password))
            .json(&serde_json::json!({ "statement": cypher }))
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;

        if let Some(message) = body.pointer("/errors/0/message").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        let columns = body
            .pointer("/data/fields")
            .and_then(Value::as_array)
            .map(|fields| {
                fields
                    .iter()
                    .map(|field| field.as_str().unwrap_or_default().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        return Ok(QueryResult {
            columns,
            rows: gridded(body.pointer("/data/values")),
            affected: None,
        });
    }

    async fn snowflake(&self, sql: &str) -> Result<QueryResult, String> {
        let body: Value = self
            .client
            .post(format!("{}/api/v2/statements", self.url))
            .bearer_auth(&self.token)
            .header(
                "X-Snowflake-Authorization-Token-Type",
                "PROGRAMMATIC_ACCESS_TOKEN",
            )
            .json(&serde_json::json!({
                "statement": sql,
                "database": self.database,
                "timeout": 60,
            }))
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;

        if body.get("data").is_none() {
            let message = body
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("snowflake refused that");

            return Err(message.to_string());
        }

        return Ok(QueryResult {
            columns: named(body.pointer("/resultSetMetaData/rowType"), "name"),
            rows: gridded(body.get("data")),
            affected: None,
        });
    }

    async fn turso(&self, sql: &str) -> Result<QueryResult, String> {
        let answer: Value = self
            .client
            .post(format!("{}/v2/pipeline", self.url))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "requests": [
                    { "type": "execute", "stmt": { "sql": sql } },
                    { "type": "close" },
                ]
            }))
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;

        if let Some(message) = answer
            .pointer("/results/0/error/message")
            .and_then(Value::as_str)
        {
            return Err(message.to_string());
        }

        let result = answer
            .pointer("/results/0/response/result")
            .ok_or_else(|| "turso answered with nothing".to_string())?;

        let columns = result
            .get("cols")
            .and_then(Value::as_array)
            .map(|cols| {
                cols.iter()
                    .map(|col| {
                        col.get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let rows = result
            .get("rows")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .map(|row| {
                        row.as_array()
                            .unwrap_or(&Vec::new())
                            .iter()
                            .map(cell_of)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        return Ok(QueryResult { columns, rows, affected: None });
    }

    async fn influx(&self, sql: &str) -> Result<QueryResult, String> {
        let answer: Value = self
            .client
            .post(format!("{}/api/v3/query_sql", self.url))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "db": self.database,
                "q": sql,
                "format": "json",
            }))
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;

        if let Some(message) = answer.get("error").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        let records = answer.as_array().cloned().unwrap_or_default();

        return Ok(records_to_grid(records));
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let listing = match self.flavour.as_str() {
            "influxdb" | "clickhouse" | "snowflake" => "show tables",
            "d1" => {
                "select name from sqlite_master where type = 'table' \
                 and name not like 'sqlite_%' order by name"
            }
            "neo4j" => "call db.labels() yield label return label",
            _ => {
                "select name from sqlite_master where type = 'table' \
                 and name not like 'sqlite_%' order by name"
            }
        };

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

fn named(value: Option<&Value>, key: &str) -> Vec<String> {
    return value
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    entry
                        .get(key)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default();
}

fn gridded(value: Option<&Value>) -> Vec<Vec<Option<String>>> {
    return value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .map(|cells| cells.iter().map(text_of).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
}

fn cell_of(value: &Value) -> Option<String> {
    if let Some(inner) = value.get("value") {
        return text_of(inner);
    }

    return text_of(value);
}

fn text_of(value: &Value) -> Option<String> {
    return match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    };
}
