use std::collections::HashSet;

use serde_json::Value;

use crate::engines::db::{hex, QueryResult, SessionConfig, TableInfo};
use crate::engines::duck::{arrow_rows, Stamps};
use crate::engines::slicing::{Filter, Op, Shape, Slice};

pub enum Driver {
    Turso(libsql::Connection),
    Click(Box<clickhouse::Client>, String),
    Neo(neo4rs::Graph),
    Snow(Box<snowflake_api::SnowflakeApi>),
    Influx(Influx),
    Influx3(Box<influxdb3_client::Client>),
    Mqtt(crate::engines::mqtt::Mqtt),
    S3(crate::engines::s3::S3),
}

pub struct Influx {
    client: influxdb2::Client,
    bucket: String,
}

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
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
            "clickhouse" => Ok(Driver::Click(
                Box::new(
                    clickhouse::Client::default()
                        .with_url(&config.url)
                        .with_user(&config.user)
                        .with_password(&config.password)
                        .with_database(if config.database.is_empty() {
                            "default"
                        } else {
                            &config.database
                        }),
                ),
                format!("gpql-{:016x}-", rand::random::<u64>()),
            )),
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
                // every call carries the org, so falling back to an empty one
                // would fail later as a puzzling rejection of the token
                let org = if config.user.is_empty() {
                    sole_org(&config.url, &config.token).await?
                } else {
                    config.user.clone()
                };

                Ok(Driver::Influx(Influx {
                    client: influxdb2::Client::new(&config.url, org, &config.token),
                    bucket: config.database.clone(),
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
            "mqtt" => Ok(Driver::Mqtt(
                crate::engines::mqtt::Mqtt::open(config).await?,
            )),
            "s3" => Ok(Driver::S3(crate::engines::s3::S3::open(config).await?)),
            other => Err(format!("{other} has no driver")),
        }
    }

    pub async fn query(&self, sql: &str, read_only: bool) -> Result<QueryResult, String> {
        match self {
            Driver::Turso(connection) => turso(connection, sql).await,
            Driver::Click(client, tag) => click(client, tag, sql, read_only).await,
            Driver::Neo(graph) => neo(graph, sql, read_only).await,
            Driver::Snow(api) => snow(api, sql).await,
            Driver::Influx(influx) => influx.flux(sql).await,
            Driver::Influx3(client) => influx3(client, sql).await,
            Driver::Mqtt(mqtt) => mqtt.query(sql).await,
            Driver::S3(s3) => s3.query(sql).await,
        }
    }

    // clickhouse takes a readonly setting per query, neo4j reads inside a
    // transaction that is rolled back, and flight sql never writes
    pub fn guarded(&self) -> bool {
        matches!(
            self,
            Driver::Click(..) | Driver::Neo(_) | Driver::Influx3(_)
        )
    }

    // each query id starts with this session's tag, so one kill reaches them all
    pub async fn cancel(&self) -> Result<(), String> {
        let Driver::Click(client, tag) = self else {
            return Err("this engine can not stop a running query".into());
        };

        client
            .query("kill query where startsWith(query_id, ?)")
            .bind(tag.as_str())
            .execute()
            .await
            .map_err(text)
    }

    pub fn binds(&self) -> bool {
        matches!(self, Driver::Turso(_))
    }

    pub async fn execute(&self, sql: &str, values: &[Option<String>]) -> Result<u64, String> {
        let Driver::Turso(connection) = self else {
            return Err("this engine takes no bound values".into());
        };

        let params = values
            .iter()
            .map(|value| match value {
                None => libsql::Value::Null,
                Some(text) => libsql::Value::Text(text.clone()),
            })
            .collect::<Vec<_>>();

        connection
            .execute(sql, libsql::params::Params::Positional(params))
            .await
            .map_err(text)
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let sql = match self {
            Driver::Turso(_) => {
                "select name from sqlite_master where type = 'table' \
                 and name not like 'sqlite_%' order by name"
            }
            Driver::Click(..) => "show tables",
            Driver::Neo(_) => "call db.labels()",
            Driver::Snow(_) => "show tables",
            Driver::Influx(influx) => return influx.tables().await,
            Driver::Mqtt(mqtt) => return Ok(mqtt.tables()),
            Driver::S3(s3) => return s3.tables().await,
            Driver::Influx3(_) => {
                "select table_name from information_schema.tables \
                 where table_schema = 'iox' order by table_name"
            }
        };

        let result = self.query(sql, true).await?;

        // snowflake lists created_on before the name, so read the name by
        // what the column is called rather than where it sits
        let at = result
            .columns
            .iter()
            .position(|column| {
                matches!(
                    column.to_ascii_lowercase().as_str(),
                    "name" | "table_name" | "label"
                )
            })
            .unwrap_or(0);

        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| row.into_iter().nth(at).flatten())
            .map(|name| TableInfo { name, rows: 0 })
            .collect())
    }

    pub async fn rows_query(&self, table: &str, slice: &Slice, shape: &Shape) -> Option<String> {
        match self {
            Driver::Influx(influx) => Some(influx.rows(table, slice, shape).await),
            Driver::S3(_) => Some(format!("ls {table}")),
            // mqtt answers only publish, so there is no command that reads a
            // topic back
            Driver::Mqtt(_) => Some(String::new()),
            _ => None,
        }
    }

    // mqtt topics appear as they are published, so the buffer reports each new
    // one and the frontend can reload its table list instead of showing a
    // snapshot taken at connect time
    pub fn on_catalog_change(&self, notify: Box<dyn Fn(&str) + Send>) {
        match self {
            Driver::Mqtt(mqtt) => mqtt.on_catalog_change(notify),
            Driver::S3(s3) => s3.on_catalog_change(notify),
            _ => {}
        }
    }

    pub fn mqtt(&self) -> Result<&crate::engines::mqtt::Mqtt, String> {
        match self {
            Driver::Mqtt(mqtt) => Ok(mqtt),
            _ => Err("not an mqtt session".to_string()),
        }
    }

    pub fn s3(&self) -> Result<&crate::engines::s3::S3, String> {
        match self {
            Driver::S3(s3) => Ok(s3),
            _ => Err("not an s3 session".to_string()),
        }
    }

    // a driver that buffers instead of querying answers a page itself; the
    // rest fall through to the generated sql
    pub async fn page(&self, table: &str, slice: &Slice) -> Option<Result<QueryResult, String>> {
        match self {
            Driver::Mqtt(mqtt) => Some(mqtt.page(table, slice)),
            Driver::S3(s3) => Some(s3.page(table, slice).await),
            _ => None,
        }
    }

    // the grid ranks and pages a whole table, so a driver only says yes once it
    // pushes the sort, the filters and the offset down to the server
    pub fn sliceable(&self) -> bool {
        !matches!(self, Driver::Neo(_))
    }

    pub async fn columns(&self) -> Result<Option<QueryResult>, String> {
        match self {
            Driver::Influx3(_) => self
                .query(
                    "select table_name, column_name, data_type, is_nullable,
                            '', '', ''
                     from information_schema.columns
                     where table_schema = 'iox'
                     order by table_name, ordinal_position",
                    true,
                )
                .await
                .map(Some),
            Driver::Influx(influx) => influx.columns().await,
            Driver::Mqtt(mqtt) => Ok(Some(mqtt.columns())),
            Driver::S3(s3) => Ok(Some(s3.columns())),
            _ => Ok(None),
        }
    }

    pub async fn databases(&self) -> Result<Vec<String>, String> {
        match self {
            Driver::Influx(influx) => Ok(influx
                .buckets()
                .await?
                .into_iter()
                .map(|bucket| bucket.name)
                .collect()),
            _ => Ok(Vec::new()),
        }
    }
}

fn some(value: &str) -> Option<&str> {
    if value.is_empty() {
        return None;
    }

    Some(value)
}

async fn turso(connection: &libsql::Connection, sql: &str) -> Result<QueryResult, String> {
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
                    Ok(libsql::Value::Blob(bytes)) => Some(hex(&bytes)),
                })
                .collect(),
        );
    }

    Ok(QueryResult {
        columns,
        rows,
        affected: None,
    })
}

async fn click(
    client: &clickhouse::Client,
    tag: &str,
    sql: &str,
    read_only: bool,
) -> Result<QueryResult, String> {
    // the client binds every bare ? itself, so one inside a string literal
    // would fail the query; ?? is how it spells a literal one
    let mut asked = client
        .query(&sql.replace('?', "??"))
        .with_setting("output_format_json_quote_decimals", "1")
        .with_setting("query_id", format!("{tag}{:016x}", rand::random::<u64>()));

    // readonly=2 still allows the setting above; 1 would refuse it
    if read_only {
        asked = asked.with_setting("readonly", "2");
    }

    let body = asked
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

    let answer: Value = serde_json::from_slice(&body).map_err(|error| error.to_string())?;

    Ok(QueryResult {
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
    })
}

#[derive(Default)]
struct Shaped {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
}

impl Shaped {
    fn add(&mut self, row: &neo4rs::Row) -> Result<(), String> {
        let fields = match row.to::<Value>() {
            Ok(Value::Object(fields)) => fields,
            Ok(other) => [("value".to_string(), other)].into_iter().collect(),
            Err(error) => return Err(format!("neo4j sent a row GPQL can not read: {error}")),
        };

        if self.columns.is_empty() {
            self.columns = fields.keys().cloned().collect();
        }

        self.rows.push(
            self.columns
                .iter()
                .map(|key| fields.get(key).and_then(cell))
                .collect(),
        );

        Ok(())
    }

    fn done(self) -> QueryResult {
        QueryResult {
            columns: self.columns,
            rows: self.rows,
            affected: None,
        }
    }
}

async fn neo(graph: &neo4rs::Graph, cypher: &str, read_only: bool) -> Result<QueryResult, String> {
    let mut shaped = Shaped::default();

    if !read_only {
        let mut stream = graph.execute(neo4rs::query(cypher)).await.map_err(text)?;

        while let Some(row) = stream.next().await.map_err(text)? {
            shaped.add(&row)?;
        }

        return Ok(shaped.done());
    }

    // this client has no read access mode, so a read-only session runs each
    // statement in a transaction that is always rolled back
    let mut txn = graph.start_txn().await.map_err(text)?;
    let walked = async {
        let mut stream = txn.execute(neo4rs::query(cypher)).await.map_err(text)?;

        while let Some(row) = stream.next(txn.handle()).await.map_err(text)? {
            shaped.add(&row)?;
        }

        Ok::<(), String>(())
    }
    .await;

    txn.rollback().await.map_err(text)?;
    walked?;

    Ok(shaped.done())
}

async fn snow(api: &snowflake_api::SnowflakeApi, sql: &str) -> Result<QueryResult, String> {
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
    let options = FormatOptions::default();

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
            .map(|array| {
                ArrayFormatter::try_new(array.as_ref(), &options).map(|printer| (array, printer))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        for index in 0..batch.num_rows() {
            rows.push(
                printers
                    .iter()
                    .map(|(array, printer)| {
                        (!array.is_null(index)).then(|| printer.value(index).to_string())
                    })
                    .collect(),
            );
        }
    }

    Ok(QueryResult {
        columns,
        rows,
        affected: None,
    })
}

async fn influx3(client: &influxdb3_client::Client, sql: &str) -> Result<QueryResult, String> {
    let answer = client.sql(sql).await.map_err(|error| error.to_string())?;

    Ok(QueryResult {
        columns: answer
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect(),
        rows: arrow_rows(answer.record_batches(), Stamps::Utc)?,
        affected: None,
    })
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

        Ok(QueryResult {
            columns,
            rows,
            affected: None,
        })
    }

    async fn buckets(&self) -> Result<Vec<TableInfo>, String> {
        const PAGE: u8 = 100;

        let mut out = Vec::new();

        loop {
            let listed = self
                .client
                .list_buckets(Some(influxdb2::api::buckets::ListBucketsRequest {
                    limit: Some(PAGE),
                    offset: Some(out.len() as u64),
                    ..Default::default()
                }))
                .await
                .map_err(refused)?;

            let count = listed.buckets.len();

            out.extend(listed.buckets.into_iter().map(|bucket| TableInfo {
                name: bucket.name,
                rows: 0,
            }));

            if count < usize::from(PAGE) {
                return Ok(out);
            }
        }
    }

    // a bucket holds the same place a database does elsewhere, so the tables
    // are its measurements once one is picked and the buckets themselves until
    // then
    async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        if self.bucket.is_empty() {
            return self.buckets().await;
        }

        Ok(self
            .measurements()
            .await?
            .into_iter()
            .map(|name| TableInfo { name, rows: 0 })
            .collect())
    }

    async fn measurements(&self) -> Result<Vec<String>, String> {
        self.client
            .list_measurements(&self.bucket, Some(SINCE), None)
            .await
            .map_err(refused)
    }

    async fn columns(&self) -> Result<Option<QueryResult>, String> {
        if self.bucket.is_empty() {
            return Ok(None);
        }

        let mut rows: Vec<Vec<Option<String>>> = Vec::new();

        for measurement in self.measurements().await? {
            let tags = self
                .client
                .list_measurement_tag_keys(&self.bucket, &measurement, Some(SINCE), None)
                .await
                .map_err(refused)?;

            let fields = self
                .client
                .list_measurement_field_keys(&self.bucket, &measurement, Some(SINCE), None)
                .await
                .map_err(refused)?;

            rows.extend(shape(&measurement, &tags, &fields));
        }

        Ok(Some(QueryResult {
            columns: Vec::new(),
            rows,
            affected: None,
        }))
    }

    async fn rows(&self, table: &str, slice: &Slice, shape: &Shape) -> String {
        let (bucket, measurement) = if self.bucket.is_empty() {
            (table, None)
        } else {
            (self.bucket.as_str(), Some(table))
        };

        let tags = match measurement {
            Some(name) if !slice.filters.is_empty() => self
                .client
                .list_measurement_tag_keys(bucket, name, Some(SINCE), None)
                .await
                .unwrap_or_default()
                .into_iter()
                .collect(),
            _ => HashSet::new(),
        };

        flux_rows(bucket, measurement, slice, shape, &tags)
    }
}

// the client defaults every schema call to the last 30 days, which silently
// hides older measurements and keys
const SINCE: &str = "0";

// influx lets one name be a tag on some points and a field on others, and the
// grid pivots them onto a single column, so the name may only be listed once
fn shape(measurement: &str, tags: &[String], fields: &[String]) -> Vec<Vec<Option<String>>> {
    let mut seen = std::collections::HashSet::new();
    let mut rows = vec![described(measurement, "_time", "time", true)];

    seen.insert("_time".to_string());

    for tag in tags.iter().filter(|tag| !tag.starts_with('_')) {
        if seen.insert(tag.clone()) {
            rows.push(described(measurement, tag, "tag", false));
        }
    }

    for field in fields {
        if seen.insert(field.clone()) {
            rows.push(described(measurement, field, "field", false));
        }
    }

    rows
}

fn described(table: &str, column: &str, sort: &str, required: bool) -> Vec<Option<String>> {
    vec![
        Some(table.to_string()),
        Some(column.to_string()),
        Some(sort.to_string()),
        Some(if required { "NO" } else { "YES" }.to_string()),
        Some(String::new()),
        Some(String::new()),
        Some(String::new()),
    ]
}

fn flux_text(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("${", "\\${");

    format!("\"{escaped}\"")
}

fn flux_number(value: &str) -> Option<String> {
    let number = value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())?;
    let written = number.to_string();

    Some(if written.contains('.') {
        written
    } else {
        format!("{written}.0")
    })
}

// flux is typed and a tag is always a string, while a field may be an int, a
// float or a string, so equality compares text and ordering compares numbers
// only when the value is one
fn flux_filter(filter: &Filter, tags: &HashSet<String>) -> String {
    let column = format!("r[{}]", flux_text(&filter.column));
    let text = flux_text(&filter.value);
    let tag = tags.contains(&filter.column)
        || matches!(filter.column.as_str(), "_measurement" | "_field");

    let ordered = |op: &str| {
        if filter.column == "_time" {
            return format!("{column} {op} time(v: {text})");
        }

        match flux_number(&filter.value).filter(|_| !tag) {
            Some(number) => format!("exists {column} and float(v: {column}) {op} {number}"),
            None => format!("exists {column} and string(v: {column}) {op} {text}"),
        }
    };

    let same = |op: &str| format!("exists {column} and string(v: {column}) {op} {text}");

    match filter.op {
        Op::IsNull => format!("not exists {column}"),
        Op::NotNull => format!("exists {column}"),
        Op::Eq => same("=="),
        Op::Ne => same("!="),
        Op::Gt => ordered(">"),
        Op::Gte => ordered(">="),
        Op::Lt => ordered("<"),
        Op::Lte => ordered("<="),
        Op::Contains => format!(
            "exists {column} and strings.containsStr(v: string(v: {column}), substr: {text})"
        ),
        Op::Starts => {
            format!("exists {column} and strings.hasPrefix(v: string(v: {column}), prefix: {text})")
        }
        Op::Ends => {
            format!("exists {column} and strings.hasSuffix(v: string(v: {column}), suffix: {text})")
        }
    }
}

fn flux_rows(
    bucket: &str,
    measurement: Option<&str>,
    slice: &Slice,
    shape: &Shape,
    tags: &HashSet<String>,
) -> String {
    let searching = slice
        .filters
        .iter()
        .any(|filter| matches!(filter.op, Op::Contains | Op::Starts | Op::Ends));

    let mut script = String::new();

    if searching {
        script.push_str("import \"strings\"\n\n");
    }

    let since = if shape.range.trim().is_empty() {
        SINCE
    } else {
        shape.range.trim()
    };

    script.push_str(&format!(
        "from(bucket: {})\n  |> range(start: {since})\n",
        flux_text(bucket)
    ));

    if let Some(name) = measurement {
        script.push_str(&format!(
            "  |> filter(fn: (r) => r._measurement == {})\n",
            flux_text(name)
        ));
    }

    // influx rolls up on the long form, while the fields are still rows
    if !shape.every.trim().is_empty() && !shape.func.trim().is_empty() {
        script.push_str(&format!(
            "  |> aggregateWindow(every: {}, fn: {}, createEmpty: false)\n",
            shape.every.trim(),
            shape.func.trim()
        ));
    }

    script.push_str(
        "  |> pivot(rowKey: [\"_time\"], columnKey: [\"_field\"], valueColumn: \"_value\")\n",
    );

    // flux keeps one table per tag set, and sort, limit and offset each apply
    // inside a table, so without this the grid ranks and pages every series
    // on its own
    script.push_str("  |> group()\n");

    for filter in &slice.filters {
        script.push_str(&format!(
            "  |> filter(fn: (r) => {})\n",
            flux_filter(filter, tags)
        ));
    }

    let (column, descending) = match &slice.sort {
        Some(sort) => (sort.column.as_str(), sort.descending),
        None => ("_time", true),
    };

    let order = if column == "_time" {
        flux_text(column)
    } else {
        format!("{}, \"_time\"", flux_text(column))
    };

    script.push_str(&format!(
        "  |> sort(columns: [{order}], desc: {descending})\n"
    ));

    script.push_str(&format!(
        "  |> limit(n: {}, offset: {})",
        slice.limit, slice.offset
    ));

    // keep comes last so a sort on a column the user left out still works
    if !slice.columns.is_empty() {
        let kept = slice
            .columns
            .iter()
            .map(|column| flux_text(column))
            .collect::<Vec<_>>()
            .join(", ");

        script.push_str(&format!("\n  |> keep(columns: [{kept}])"));
    }

    script
}

async fn sole_org(url: &str, token: &str) -> Result<String, String> {
    let listed = influxdb2::Client::new(url, "", token)
        .list_organizations(influxdb2::api::organization::ListOrganizationRequest {
            limit: Some(100),
            ..Default::default()
        })
        .await
        .map_err(refused)?;

    let mut names: Vec<String> = listed.orgs.into_iter().map(|org| org.name).collect();

    names.sort();

    match names.len() {
        0 => Err("that token can not see any organisation".to_string()),
        1 => Ok(names.remove(0)),
        _ => Err(format!(
            "that token can see several organisations ({}), so name the one to use",
            names.join(", ")
        )),
    }
}

fn refused(error: influxdb2::RequestError) -> String {
    let text = error.to_string();

    // influx only ever says "unauthorized access" here, so name the two things
    // worth checking, and keep its own words after them
    if text.contains("401 Unauthorized") {
        return format!(
            "influx refused that API token, or it cannot read this bucket: {}",
            said(&text).unwrap_or_else(|| text.clone())
        );
    }

    said(&text)
        .map(|message| format!("{message} ({text})"))
        .unwrap_or(text)
}

// influx wraps the real complaint in a json body; anything around it is noise
// beside a form field
fn said(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    let body: Value = serde_json::from_str(text.get(start..=end)?).ok()?;

    body.get("message")?.as_str().map(str::to_string)
}

fn noise(column: &str) -> bool {
    matches!(column, "result" | "table" | "_start" | "_stop")
}

fn place(column: &str) -> (u8, String) {
    let rank = match column {
        "_time" => 0,
        "_measurement" => 1,
        _ if column.starts_with('_') => 3,
        _ => 2,
    };

    (rank, column.to_string())
}

fn reading(value: &influxdb2_structmap::value::Value) -> Option<String> {
    use influxdb2_structmap::value::Value as Reading;

    match value {
        Reading::Unknown => None,
        Reading::String(text) => Some(text.clone()),
        Reading::Double(number) => Some(number.to_string()),
        Reading::Bool(flag) => Some(flag.to_string()),
        Reading::Long(number) => Some(number.to_string()),
        Reading::UnsignedLong(number) => Some(number.to_string()),
        Reading::Duration(span) => Some(span.to_string()),
        Reading::Base64Binary(bytes) => Some(hex(bytes)),
        Reading::TimeRFC(stamp) => Some(stamp.to_rfc3339()),
    }
}

fn grid(value: Option<&Value>) -> Vec<Vec<Option<String>>> {
    value
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
        .unwrap_or_default()
}

fn cell(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod flux {
    use super::*;
    use crate::engines::slicing::Sort;

    #[test]
    fn pages_a_measurement() {
        let slice = Slice {
            limit: 50,
            offset: 100,
            sort: Some(Sort {
                column: "load".into(),
                descending: false,
            }),
            filters: vec![Filter {
                column: "host".into(),
                op: Op::Contains,
                value: "edge".into(),
            }],
            columns: vec!["_time".into(), "usage".into()],
        };

        let tags = HashSet::from(["host".to_string()]);
        let script = flux_rows("metrics", Some("cpu"), &slice, &Shape::default(), &tags);

        assert!(script.starts_with("import \"strings\""));
        assert!(script.contains(r#"from(bucket: "metrics")"#));
        assert!(script.contains(r#"r._measurement == "cpu""#));
        assert!(script.contains("|> group()"));
        assert!(script.contains(r#"strings.containsStr(v: string(v: r["host"]), substr: "edge")"#));
        assert!(script.contains(r#"sort(columns: ["load", "_time"], desc: false)"#));
        assert!(script.contains("limit(n: 50, offset: 100)"));
        assert!(script.ends_with(r#"keep(columns: ["_time", "usage"])"#));
    }

    // influx 2 runs only what the classifier calls a read while read only, so
    // every script the grid writes has to come out as one
    #[test]
    fn every_generated_script_is_a_read() {
        use crate::engines::access::{access, Access};

        let tags = HashSet::from(["host".to_string()]);

        for column in ["host", "temp", "_time"] {
            for op in [Op::Eq, Op::Gt, Op::Contains, Op::Ends, Op::IsNull] {
                let slice = Slice {
                    limit: 10,
                    sort: Some(Sort {
                        column: column.into(),
                        descending: false,
                    }),
                    filters: vec![Filter {
                        column: column.into(),
                        op,
                        value: "it's \"${x}\" \\ 12".into(),
                    }],
                    columns: vec!["_time".into(), column.into()],
                    ..Default::default()
                };
                let script = flux_rows("b", Some("m"), &slice, &Shape::default(), &tags);

                assert_eq!(access("influxdb2", &script), Access::Read, "{script}");
            }
        }
    }

    #[test]
    fn browses_a_whole_bucket() {
        let asked = Slice {
            limit: 10,
            ..Default::default()
        };
        let script = flux_rows("metrics", None, &asked, &Shape::default(), &HashSet::new());

        assert!(!script.contains("_measurement =="));
        assert!(!script.contains("import"));
        assert!(script.contains(r#"sort(columns: ["_time"], desc: true)"#));
    }

    #[test]
    fn a_name_that_is_both_a_tag_and_a_field_is_listed_once() {
        let tags = vec!["_measurement".to_string(), "control_target_id".to_string()];
        let fields = vec!["control_target_id".to_string(), "temp".to_string()];

        let named: Vec<String> = shape("sensor", &tags, &fields)
            .into_iter()
            .map(|row| row[1].clone().unwrap_or_default())
            .collect();

        assert_eq!(named, ["_time", "control_target_id", "temp"]);
    }

    #[test]
    fn an_influx_complaint_is_read_out_of_its_json() {
        let bad_org = "HTTP request returned an error: 400 Bad Request,              `{\"code\":\"invalid\",\"message\":\"failed to decode request body:              organization name \\\"nosuchorg\\\" not found\"}`";

        assert!(said(bad_org).unwrap().contains("not found"));
        assert!(said("no json here").is_none());
    }

    #[test]
    fn a_tag_is_compared_as_text_and_only_a_finite_number_as_a_number() {
        let tags = HashSet::from(["id".to_string()]);
        let asked = |column: &str, op: Op, value: &str| {
            flux_filter(
                &Filter {
                    column: column.into(),
                    op,
                    value: value.into(),
                },
                &tags,
            )
        };

        assert_eq!(
            asked("id", Op::Eq, "42"),
            r#"exists r["id"] and string(v: r["id"]) == "42""#
        );
        assert_eq!(
            asked("id", Op::Gt, "42"),
            r#"exists r["id"] and string(v: r["id"]) > "42""#
        );
        assert_eq!(
            asked("temp", Op::Gt, "12"),
            r#"exists r["temp"] and float(v: r["temp"]) > 12.0"#
        );
        assert_eq!(
            asked("temp", Op::Lt, "inf"),
            r#"exists r["temp"] and string(v: r["temp"]) < "inf""#
        );
        assert_eq!(
            asked("host", Op::Eq, "${x}"),
            r#"exists r["host"] and string(v: r["host"]) == "\${x}""#
        );
    }
}

#[cfg(test)]
mod live {
    use super::*;
    use crate::engines::db::{open, Engine};
    use crate::engines::slicing::Sort;

    // GPQL_TEST_INFLUX2=url|org|token|bucket
    async fn open_test() -> Option<crate::engines::db::Session> {
        let target = std::env::var("GPQL_TEST_INFLUX2").ok()?;
        let mut parts = target.splitn(4, '|');

        let config = SessionConfig {
            kind: "influxdb2".into(),
            url: parts.next()?.into(),
            user: parts.next()?.into(),
            token: parts.next()?.into(),
            database: parts.next()?.into(),
            read_only: true,
            ..Default::default()
        };

        Some(open(&config).await.expect("could not open the test bucket"))
    }

    fn driver(session: &crate::engines::db::Session) -> &Driver {
        match &session.engine {
            Engine::Driver(driver) => driver,
            _ => panic!("influx did not open as a driver"),
        }
    }

    #[tokio::test]
    async fn a_bucket_lists_its_measurements_as_tables() {
        let Some(session) = open_test().await else {
            return;
        };

        let names: Vec<String> = driver(&session)
            .tables()
            .await
            .unwrap()
            .into_iter()
            .map(|table| table.name)
            .collect();

        assert!(names.contains(&"sensor".to_string()), "{names:?}");
    }

    #[tokio::test]
    async fn tags_and_fields_come_back_as_columns() {
        let Some(session) = open_test().await else {
            return;
        };

        let listing = driver(&session).columns().await.unwrap().unwrap();
        let of_sensor: Vec<(String, String)> = listing
            .rows
            .iter()
            .filter(|row| row[0].as_deref() == Some("sensor"))
            .map(|row| {
                (
                    row[1].clone().unwrap_or_default(),
                    row[2].clone().unwrap_or_default(),
                )
            })
            .collect();

        assert!(
            of_sensor.contains(&("_time".into(), "time".into())),
            "{of_sensor:?}"
        );
        assert!(
            of_sensor.contains(&("house".into(), "tag".into())),
            "{of_sensor:?}"
        );
        assert!(
            of_sensor.contains(&("temp".into(), "field".into())),
            "{of_sensor:?}"
        );
    }

    #[tokio::test]
    async fn rows_arrive_pivoted_and_paged() {
        let Some(session) = open_test().await else {
            return;
        };

        let ask = |limit: u32, offset: u32| Slice {
            limit,
            offset,
            sort: Some(Sort {
                column: "_time".into(),
                descending: false,
            }),
            ..Default::default()
        };

        let page = crate::engines::slicing::table_rows(&session, "sensor", &ask(1, 0))
            .await
            .unwrap();

        assert_eq!(page.rows.len(), 1);
        assert!(
            page.columns.contains(&"temp".to_string()),
            "{:?}",
            page.columns
        );

        let next = crate::engines::slicing::table_rows(&session, "sensor", &ask(1, 1))
            .await
            .unwrap();

        assert_eq!(next.rows.len(), 1);
        assert_ne!(page.rows[0], next.rows[0], "offset did not move the window");
    }

    #[tokio::test]
    async fn a_filter_reaches_the_server() {
        let Some(session) = open_test().await else {
            return;
        };

        let slice = Slice {
            limit: 100,
            filters: vec![Filter {
                column: "house".into(),
                op: Op::Eq,
                value: "b".into(),
            }],
            ..Default::default()
        };

        let page = crate::engines::slicing::table_rows(&session, "sensor", &slice)
            .await
            .unwrap();
        let at = page
            .columns
            .iter()
            .position(|name| name == "house")
            .unwrap();

        assert!(!page.rows.is_empty());
        assert!(page.rows.iter().all(|row| row[at].as_deref() == Some("b")));
    }
}
