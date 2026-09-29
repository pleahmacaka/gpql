use redis::aio::MultiplexedConnection;
use redis::IntoConnectionInfo;
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

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

// a gateway in front of the api can answer in html or plain text, and that
// text is the only clue to what went wrong
async fn answer(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let body = response.text().await.map_err(text)?;

    match serde_json::from_str(&body) {
        Ok(value) => Ok(value),
        Err(_) if status.is_success() => Err(format!("the server sent back: {body}")),
        Err(_) => Err(format!("{status}: {body}")),
    }
}

impl Http {
    pub fn open(config: &SessionConfig) -> Self {
        Http {
            flavour: config.kind.clone(),
            url: config.url.trim_end_matches('/').to_string(),
            token: config.token.clone(),
            database: config.database.clone(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn query(&self, sql: &str, read_only: bool) -> Result<QueryResult, String> {
        match self.flavour.as_str() {
            "supabase_api" => self.supabase(sql, read_only).await,
            _ => self.d1(sql).await,
        }
    }

    async fn d1(&self, sql: &str) -> Result<QueryResult, String> {
        let endpoint = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/d1/database/{}/query",
            self.url.trim_start_matches("https://"),
            self.database
        );

        let response = self
            .client
            .post(endpoint)
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "sql": sql }))
            .send()
            .await
            .map_err(text)?;
        let body = answer(response).await?;

        if let Some(message) = body.pointer("/errors/0/message").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        let records = body
            .pointer("/result/0/results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        Ok(records_to_grid(records))
    }

    async fn supabase(&self, sql: &str, read_only: bool) -> Result<QueryResult, String> {
        let endpoint = format!(
            "https://api.supabase.com/v1/projects/{}/database/query",
            self.database
        );

        let response = self
            .client
            .post(endpoint)
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "query": sql, "read_only": read_only }))
            .send()
            .await
            .map_err(text)?;
        let body = answer(response).await?;

        if let Some(message) = body.get("message").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        let records = body.as_array().cloned().unwrap_or_default();

        Ok(records_to_grid(records))
    }

    pub async fn columns(&self) -> Result<QueryResult, String> {
        let listing = self
            .query(
                "select c.table_name, c.column_name,
                        format_type(ta.atttypid, ta.atttypmod) as data_type,
                        c.is_nullable,
                        case when pk.attname is null then '' else 'PRI' end
                          as column_key,
                        fk.target_table, fk.target_column
                 from information_schema.columns c
                 join pg_attribute ta
                   on ta.attrelid = format('%I.%I', c.table_schema, c.table_name)::regclass
                  and ta.attname = c.column_name
                 left join (
                   select t.relname as table_name, a.attname
                   from pg_constraint k
                   join pg_class t on t.oid = k.conrelid
                   join pg_namespace n on n.oid = t.relnamespace
                   join pg_attribute a
                     on a.attrelid = k.conrelid and a.attnum = any(k.conkey)
                   where k.contype = 'p' and n.nspname = 'public'
                 ) pk on pk.table_name = c.table_name
                     and pk.attname = c.column_name
                 left join (
                   select distinct on (t.relname, a.attname)
                          t.relname as table_name, a.attname,
                          case when r.relnamespace = t.relnamespace
                               then r.relname
                               else rn.nspname || '.' || r.relname
                          end as target_table,
                          ra.attname as target_column
                   from pg_constraint k
                   join pg_class t on t.oid = k.conrelid
                   join pg_namespace n on n.oid = t.relnamespace
                   join pg_class r on r.oid = k.confrelid
                   join pg_namespace rn on rn.oid = r.relnamespace
                   cross join lateral unnest(k.conkey, k.confkey) as pair(own, other)
                   join pg_attribute a
                     on a.attrelid = k.conrelid and a.attnum = pair.own
                   join pg_attribute ra
                     on ra.attrelid = k.confrelid and ra.attnum = pair.other
                   where k.contype = 'f' and n.nspname = 'public'
                   order by t.relname, a.attname, k.conname
                 ) fk on fk.table_name = c.table_name
                     and fk.attname = c.column_name
                 where c.table_schema = 'public'
                 order by c.table_name, c.ordinal_position",
                true,
            )
            .await?;

        Ok(ordered(
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
        ))
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let listing = if self.flavour == "supabase_api" {
            "select c.relname as name
             from pg_class c
             join pg_namespace n on n.oid = c.relnamespace
             where c.relkind in ('r', 'p') and not c.relispartition
               and n.nspname = 'public'
             order by c.relname"
        } else {
            "select name from sqlite_master where type = 'table' \
             and name not like 'sqlite_%' order by name"
        };

        let result = self.query(listing, true).await?;
        let column = result
            .columns
            .iter()
            .position(|name| name == "table_name" || name == "name" || name == "label")
            .unwrap_or(0);

        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| row.get(column).cloned().flatten())
            .map(|name| TableInfo { name, rows: 0 })
            .collect())
    }
}

impl Graph {
    pub async fn open(config: &SessionConfig) -> Result<Self, String> {
        crate::engines::db::install_crypto();

        let mut info = config.url.as_str().into_connection_info().map_err(text)?;

        if !config.user.is_empty() {
            info.redis.username = Some(config.user.clone());
        }

        if !config.password.is_empty() {
            info.redis.password = Some(config.password.clone());
        }

        let connection = redis::Client::open(info)
            .map_err(text)?
            .get_multiplexed_async_connection()
            .await
            .map_err(text)?;

        Ok(Graph {
            name: if config.database.is_empty() {
                "falkordb".into()
            } else {
                config.database.clone()
            },
            connection: Mutex::new(connection),
        })
    }

    pub async fn query(&self, cypher: &str, read_only: bool) -> Result<QueryResult, String> {
        let mut connection = self.connection.lock().await;
        let command = if read_only {
            "GRAPH.RO_QUERY"
        } else {
            "GRAPH.QUERY"
        };

        let answer: redis::Value = redis::cmd(command)
            .arg(&self.name)
            .arg(cypher)
            .query_async(&mut *connection)
            .await
            .map_err(text)?;

        Ok(shape(answer))
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let result = self.query("call db.labels()", true).await?;

        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| row.into_iter().next().flatten())
            .map(|name| TableInfo { name, rows: 0 })
            .collect())
    }
}

// a verbose reply is [header, rows, statistics]; a statement that returns
// nothing answers with the statistics alone
fn shape(answer: redis::Value) -> QueryResult {
    let redis::Value::Array(mut parts) = answer else {
        return QueryResult {
            columns: vec!["result".into()],
            rows: vec![vec![scalar(&answer)]],
            affected: None,
        };
    };

    if parts.len() < 3 {
        let stats = match parts.pop() {
            Some(redis::Value::Array(lines)) => lines,
            Some(other) => vec![other],
            None => Vec::new(),
        };

        return QueryResult {
            columns: vec!["result".into()],
            rows: stats.iter().map(|line| vec![scalar(line)]).collect(),
            affected: None,
        };
    }

    let columns = match &parts[0] {
        redis::Value::Array(names) => names
            .iter()
            .map(|name| match name {
                redis::Value::Array(pair) => pair.last().and_then(scalar).unwrap_or_default(),
                other => scalar(other).unwrap_or_default(),
            })
            .collect(),
        _ => Vec::new(),
    };

    let rows = match &parts[1] {
        redis::Value::Array(rows) => rows
            .iter()
            .map(|row| match row {
                redis::Value::Array(cells) => cells.iter().map(scalar).collect(),
                other => vec![scalar(other)],
            })
            .collect(),
        _ => Vec::new(),
    };

    QueryResult {
        columns,
        rows,
        affected: None,
    }
}

fn scalar(value: &redis::Value) -> Option<String> {
    match json(value) {
        Value::Null => None,
        Value::String(text) => Some(text),
        other => Some(other.to_string()),
    }
}

// a node comes back as [["id", 1], ["labels", [..]], ["properties", [..]]],
// so any list made only of named pairs reads better as an object
fn json(value: &redis::Value) -> Value {
    match value {
        redis::Value::Nil => Value::Null,
        redis::Value::Int(number) => Value::from(*number),
        redis::Value::Double(number) => Value::from(*number),
        redis::Value::Boolean(flag) => Value::from(*flag),
        redis::Value::BulkString(bytes) => Value::from(String::from_utf8_lossy(bytes).into_owned()),
        redis::Value::SimpleString(text) => Value::from(text.clone()),
        redis::Value::Array(items) => {
            let named = |item: &redis::Value| match item {
                redis::Value::Array(pair) if pair.len() == 2 => match &pair[0] {
                    redis::Value::BulkString(bytes) => {
                        Some((String::from_utf8_lossy(bytes).into_owned(), json(&pair[1])))
                    }
                    redis::Value::SimpleString(key) => Some((key.clone(), json(&pair[1]))),
                    _ => None,
                },
                _ => None,
            };

            match items
                .iter()
                .map(named)
                .collect::<Option<serde_json::Map<_, _>>>()
            {
                Some(object) if !items.is_empty() => Value::Object(object),
                _ => Value::Array(items.iter().map(json).collect()),
            }
        }
        redis::Value::Map(pairs) => Value::Object(
            pairs
                .iter()
                .map(|(key, item)| (scalar(key).unwrap_or_default(), json(item)))
                .collect(),
        ),
        other => Value::from(format!("{other:?}")),
    }
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

    QueryResult {
        columns: wanted.iter().map(|name| name.to_string()).collect(),
        rows,
        affected: None,
    }
}

fn records_to_grid(records: Vec<Value>) -> QueryResult {
    let mut columns: Vec<String> = Vec::new();

    for record in &records {
        for key in record
            .as_object()
            .map(|map| map.keys())
            .into_iter()
            .flatten()
        {
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

    QueryResult {
        columns,
        rows,
        affected: None,
    }
}

fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod falkor {
    use super::*;

    fn bulk(text: &str) -> redis::Value {
        redis::Value::BulkString(text.as_bytes().to_vec())
    }

    #[test]
    fn a_verbose_reply_keeps_names_and_values_apart() {
        let node = redis::Value::Array(vec![
            redis::Value::Array(vec![bulk("id"), redis::Value::Int(0)]),
            redis::Value::Array(vec![
                bulk("labels"),
                redis::Value::Array(vec![bulk("Person")]),
            ]),
            redis::Value::Array(vec![
                bulk("properties"),
                redis::Value::Array(vec![redis::Value::Array(vec![bulk("name"), bulk("Alice")])]),
            ]),
        ]);
        let reply = redis::Value::Array(vec![
            redis::Value::Array(vec![bulk("name"), bulk("n")]),
            redis::Value::Array(vec![redis::Value::Array(vec![bulk("Alice"), node])]),
            redis::Value::Array(vec![bulk("Cached execution: 0")]),
        ]);

        let result = shape(reply);

        assert_eq!(result.columns, ["name", "n"]);
        assert_eq!(result.rows[0][0].as_deref(), Some("Alice"));

        let node: Value = serde_json::from_str(result.rows[0][1].as_deref().unwrap()).unwrap();

        assert_eq!(node["labels"][0], "Person");
        assert_eq!(node["properties"]["name"], "Alice");
    }

    #[test]
    fn a_reply_with_only_statistics_lists_them() {
        let reply = redis::Value::Array(vec![redis::Value::Array(vec![bulk("Nodes created: 1")])]);

        let result = shape(reply);

        assert_eq!(
            result.rows,
            vec![vec![Some("Nodes created: 1".to_string())]]
        );
    }
}
