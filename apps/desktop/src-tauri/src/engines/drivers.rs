use serde_json::Value;

use crate::engines::db::{QueryResult, SessionConfig, TableInfo};

pub enum Driver {
    Turso(libsql::Connection),
    Click(Box<clickhouse::Client>),
    Neo(neo4rs::Graph),
    Snow(Box<snowflake_api::SnowflakeApi>),
    Influx(Influx),
    Influx3(Box<influxdb3_client::Client>),
}

pub struct Influx {
    client: influxdb2::Client,
}

impl Driver {
    pub async fn open(config: &SessionConfig) -> Result<Self, String> {
        match config.kind.as_str() {
            "turso" => {
                let database =
                    libsql::Builder::new_remote(config.url.clone(), config.token.clone())
                        .build()
                        .await
                        .map_err(|error| error.to_string())?;

                Ok(Driver::Turso(
                    database.connect().map_err(|error| error.to_string())?,
                ))
            }
            "clickhouse" => Ok(Driver::Click(Box::new(
                clickhouse::Client::default()
                    .with_url(&config.url)
                    .with_user(&config.user)
                    .with_password(&config.password)
                    .with_database(if config.database.is_empty() {
                        "default"
                    } else {
                        &config.database
                    }),
            ))),
            "neo4j" => Ok(Driver::Neo(
                neo4rs::Graph::new(&config.url, &config.user, &config.password)
                    .await
                    .map_err(|error| error.to_string())?,
            )),
            "snowflake" => Ok(Driver::Snow(Box::new(
                snowflake_api::SnowflakeApi::with_password_auth(
                    &config.host,
                    some(&config.warehouse),
                    some(&config.database),
                    some(&config.schema),
                    &config.user,
                    None,
                    &config.password,
                )
                .map_err(|error| error.to_string())?,
            ))),
            "influxdb2" => {
                let org = if config.user.is_empty() {
                    sole_org(&config.url, &config.token).await.unwrap_or_default()
                } else {
                    config.user.clone()
                };

                Ok(Driver::Influx(Influx {
                    client: influxdb2::Client::new(&config.url, org, &config.token),
                }))
            }
            "influxdb" => {
                let settings = influxdb3_client::ClientConfig::builder()
                    .host(&config.url)
                    .token(&config.token)
                    .database(&config.database)
                    .build()
                    .map_err(|error| error.to_string())?;

                Ok(Driver::Influx3(Box::new(
                    influxdb3_client::Client::new(settings)
                        .await
                        .map_err(|error| error.to_string())?,
                )))
            }
            other => Err(format!("{other} has no driver")),
        }
    }

    pub async fn query(&self, sql: &str) -> Result<QueryResult, String> {
        match self {
            Driver::Turso(connection) => turso(connection, sql).await,
            Driver::Click(client) => click(client, sql).await,
            Driver::Neo(graph) => neo(graph, sql).await,
            Driver::Snow(api) => snow(api, sql).await,
            Driver::Influx(influx) => influx.flux(sql).await,
            Driver::Influx3(client) => influx3(client, sql).await,
        }
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let sql = match self {
            Driver::Turso(_) => {
                "select name from sqlite_master where type = 'table' \
                 and name not like 'sqlite_%' order by name"
            }
            Driver::Click(_) => "show tables",
            Driver::Neo(_) => "call db.labels()",
            Driver::Snow(_) => "show tables",
            Driver::Influx(influx) => return influx.buckets().await,
            Driver::Influx3(_) => {
                "select table_name from information_schema.tables \
                 where table_schema = 'iox' order by table_name"
            }
        };

        let result = self.query(sql).await?;

        return Ok(result
            .rows
            .into_iter()
            .filter_map(|row| row.into_iter().next().flatten())
            .map(|name| TableInfo { name, rows: 0 })
            .collect());
    }

    pub fn rows_query(&self, table: &str, limit: u32) -> Option<String> {
        match self {
            Driver::Neo(_) => Some(format!("match (n:{table}) return n limit {limit}")),
            Driver::Influx(_) => Some(format!(
                r#"from(bucket: "{table}")
  |> range(start: 0)
  |> pivot(rowKey: ["_time"], columnKey: ["_field"], valueColumn: "_value")
  |> sort(columns: ["_time"], desc: true)
  |> limit(n: {limit})"#
            )),
            _ => None,
        }
    }
}

fn some(value: &str) -> Option<&str> {
    if value.is_empty() {
        return None;
    }

    return Some(value);
}

async fn turso(
    connection: &libsql::Connection,
    sql: &str,
) -> Result<QueryResult, String> {
    let mut answer = connection
        .query(sql, ())
        .await
        .map_err(|error| error.to_string())?;

    let columns = (0..answer.column_count())
        .map(|index| answer.column_name(index).unwrap_or_default().to_string())
        .collect::<Vec<_>>();

    let mut rows: Vec<Vec<Option<String>>> = Vec::new();

    while let Some(row) = answer.next().await.map_err(|error| error.to_string())? {
        rows.push(
            (0..columns.len() as i32)
                .map(|index| match row.get_value(index) {
                    Ok(libsql::Value::Null) | Err(_) => None,
                    Ok(libsql::Value::Integer(number)) => Some(number.to_string()),
                    Ok(libsql::Value::Real(number)) => Some(number.to_string()),
                    Ok(libsql::Value::Text(text)) => Some(text),
                    Ok(libsql::Value::Blob(bytes)) => {
                        Some(format!("{} bytes", bytes.len()))
                    }
                })
                .collect(),
        );
    }

    return Ok(QueryResult { columns, rows, affected: None });
}

async fn click(client: &clickhouse::Client, sql: &str) -> Result<QueryResult, String> {
    let body = client
        .query(sql)
        .fetch_bytes("JSONCompact")
        .map_err(|error| error.to_string())?
        .collect()
        .await
        .map_err(|error| error.to_string())?;

    if body.is_empty() {
        return Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected: None,
        });
    }

    let answer: Value =
        serde_json::from_slice(&body).map_err(|error| error.to_string())?;

    return Ok(QueryResult {
        columns: answer
            .get("meta")
            .and_then(Value::as_array)
            .map(|meta| {
                meta.iter()
                    .map(|column| {
                        column
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string()
                    })
                    .collect()
            })
            .unwrap_or_default(),
        rows: grid(answer.get("data")),
        affected: None,
    });
}

async fn neo(graph: &neo4rs::Graph, cypher: &str) -> Result<QueryResult, String> {
    let mut stream = graph
        .execute(neo4rs::query(cypher))
        .await
        .map_err(|error| error.to_string())?;

    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Option<String>>> = Vec::new();

    while let Some(row) = stream.next().await.map_err(|error| error.to_string())? {
        let Ok(Value::Object(fields)) = row.to::<Value>() else {
            continue;
        };

        if columns.is_empty() {
            columns = fields.keys().cloned().collect();
        }

        rows.push(
            columns
                .iter()
                .map(|key| fields.get(key).and_then(cell))
                .collect(),
        );
    }

    return Ok(QueryResult { columns, rows, affected: None });
}

async fn snow(
    api: &snowflake_api::SnowflakeApi,
    sql: &str,
) -> Result<QueryResult, String> {
    use arrow_cast::display::{ArrayFormatter, FormatOptions};

    let answer = api.exec(sql).await.map_err(|error| error.to_string())?;

    let batches = match answer {
        snowflake_api::QueryResult::Arrow(batches) => batches,
        snowflake_api::QueryResult::Json(json) => {
            return Ok(QueryResult {
                columns: json.schema.iter().map(|field| field.name.clone()).collect(),
                rows: grid(Some(&json.value)),
                affected: None,
            });
        }
        snowflake_api::QueryResult::Empty => {
            return Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected: None,
            });
        }
    };

    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Option<String>>> = Vec::new();
    let options = FormatOptions::default().with_null("");

    for batch in &batches {
        if columns.is_empty() {
            columns = batch
                .schema()
                .fields()
                .iter()
                .map(|field| field.name().clone())
                .collect();
        }

        let printers = batch
            .columns()
            .iter()
            .map(|array| ArrayFormatter::try_new(array.as_ref(), &options))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        for index in 0..batch.num_rows() {
            rows.push(
                printers
                    .iter()
                    .map(|printer| {
                        let text = printer.value(index).to_string();

                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    })
                    .collect(),
            );
        }
    }

    return Ok(QueryResult { columns, rows, affected: None });
}

async fn influx3(
    client: &influxdb3_client::Client,
    sql: &str,
) -> Result<QueryResult, String> {
    let answer = client.sql(sql).await.map_err(|error| error.to_string())?;

    let columns = answer
        .column_names()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();

    let rows = answer
        .rows()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|row| row.values().iter().map(point).collect())
        .collect();

    return Ok(QueryResult { columns, rows, affected: None });
}

fn point(value: &influxdb3_client::Value) -> Option<String> {
    use influxdb3_client::Value as Point;

    return match value {
        Point::Null => None,
        Point::Bool(flag) => Some(flag.to_string()),
        Point::I8(number) => Some(number.to_string()),
        Point::I16(number) => Some(number.to_string()),
        Point::I32(number) => Some(number.to_string()),
        Point::I64(number) | Point::Timestamp(number) => Some(number.to_string()),
        Point::U8(number) => Some(number.to_string()),
        Point::U16(number) => Some(number.to_string()),
        Point::U32(number) => Some(number.to_string()),
        Point::U64(number) => Some(number.to_string()),
        Point::F32(number) => Some(number.to_string()),
        Point::F64(number) => Some(number.to_string()),
        Point::String(text) => Some(text.clone()),
        Point::Binary(bytes) => Some(format!("{} bytes", bytes.len())),
    };
}

impl Influx {
    async fn flux(&self, script: &str) -> Result<QueryResult, String> {
        let asked = std::panic::AssertUnwindSafe(
            self.client
                .query_raw(Some(influxdb2::models::Query::new(script.to_string()))),
        );

        let records = futures_util::FutureExt::catch_unwind(asked)
            .await
            .map_err(|_| "influx sent back a row this client can not read".to_string())?
            .map_err(refused)?;

        let mut columns: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<Option<String>>> = Vec::new();

        for record in &records {
            for key in record.values.keys() {
                if noise(key) || columns.contains(key) {
                    continue;
                }

                columns.push(key.clone());
            }
        }

        columns.sort_by_key(|name| place(name));

        for record in &records {
            rows.push(
                columns
                    .iter()
                    .map(|key| record.values.get(key).and_then(reading))
                    .collect(),
            );
        }

        return Ok(QueryResult { columns, rows, affected: None });
    }

    async fn buckets(&self) -> Result<Vec<TableInfo>, String> {
        let listed = self
            .client
            .list_buckets(Some(influxdb2::api::buckets::ListBucketsRequest {
                limit: Some(100),
                ..Default::default()
            }))
            .await
            .map_err(refused)?;

        return Ok(listed
            .buckets
            .into_iter()
            .map(|bucket| TableInfo { name: bucket.name, rows: 0 })
            .collect());
    }
}

async fn sole_org(url: &str, token: &str) -> Result<String, String> {
    let listed = influxdb2::Client::new(url, "", token)
        .list_organizations(influxdb2::api::organization::ListOrganizationRequest {
            limit: Some(1),
            ..Default::default()
        })
        .await
        .map_err(refused)?;

    return listed
        .orgs
        .into_iter()
        .next()
        .map(|org| org.name)
        .ok_or_else(|| "that token can not see any organisation".to_string());
}

fn refused(error: influxdb2::RequestError) -> String {
    let text = error.to_string();

    if text.contains("401") || text.contains("unauthorized") {
        return "influx turned that token down. InfluxDB 2 authorises the API                 with an API token, not the user and password you type into the                 web UI."
            .to_string();
    }

    return text;
}

fn noise(column: &str) -> bool {
    return matches!(column, "result" | "table" | "_start" | "_stop");
}

fn place(column: &str) -> (u8, String) {
    let rank = match column {
        "_time" => 0,
        "_measurement" => 1,
        _ if column.starts_with('_') => 3,
        _ => 2,
    };

    return (rank, column.to_string());
}

fn reading(value: &influxdb2_structmap::value::Value) -> Option<String> {
    use influxdb2_structmap::value::Value as Reading;

    return match value {
        Reading::Unknown => None,
        Reading::String(text) => Some(text.clone()),
        Reading::Double(number) => Some(number.to_string()),
        Reading::Bool(flag) => Some(flag.to_string()),
        Reading::Long(number) => Some(number.to_string()),
        Reading::UnsignedLong(number) => Some(number.to_string()),
        Reading::Duration(span) => Some(span.to_string()),
        Reading::Base64Binary(bytes) => Some(format!("{} bytes", bytes.len())),
        Reading::TimeRFC(stamp) => Some(stamp.to_rfc3339()),
    };
}

fn grid(value: Option<&Value>) -> Vec<Vec<Option<String>>> {
    return value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .map(|cells| cells.iter().map(cell).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
}

fn cell(value: &Value) -> Option<String> {
    return match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    };
}


