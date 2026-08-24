use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use tokio_postgres::{NoTls, SimpleQueryMessage};
use tokio_postgres_rustls::MakeRustlsConnect;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub kind: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub tls: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHandle {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub kind: String,
    pub read_only: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub affected: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub name: String,
    pub rows: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub primary_key: bool,
    pub required: bool,
    pub references: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableSchema {
    pub name: String,
    pub rows: i64,
    pub columns: Vec<ColumnInfo>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub hints: Vec<String>,
}

pub enum Engine {
    Postgres(tokio_postgres::Client),
    MySql(crate::mysql::MySql),
    Duck(crate::duck::Duck),
    Sqlite(Mutex<Connection>),
    Http(crate::remote::Http),
    Graph(crate::remote::Graph),
}

pub struct Session {
    pub engine: Engine,
    pub read_only: bool,
    pub label: String,
    pub detail: String,
    pub kind: String,
}

#[derive(Default)]
pub struct Sessions {
    open: Mutex<HashMap<String, Arc<Session>>>,
    next: AtomicU64,
}

impl Sessions {
    pub fn insert(&self, session: Session) -> SessionHandle {
        let id = format!("s{}", self.next.fetch_add(1, Ordering::Relaxed));
        let handle = SessionHandle {
            id: id.clone(),
            label: session.label.clone(),
            detail: session.detail.clone(),
            kind: session.kind.clone(),
            read_only: session.read_only,
        };

        self.open.lock().unwrap().insert(id, Arc::new(session));

        return handle;
    }

    pub fn get(&self, id: &str) -> Result<Arc<Session>, String> {
        self.open
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| "that session is closed".to_string())
    }

    pub fn remove(&self, id: &str) {
        self.open.lock().unwrap().remove(id);
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn pg_conn_string(config: &SessionConfig) -> String {
    let mut out = String::new();

    let mut push = |key: &str, value: &str| {
        if value.is_empty() {
            return;
        }
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
        out.push_str(&format!("{key}='{escaped}' "));
    };

    push("host", if config.host.is_empty() { "127.0.0.1" } else { &config.host });
    push("port", if config.port.is_empty() { "5432" } else { &config.port });
    push("user", &config.user);
    push("password", &config.password);
    push("dbname", &config.database);
    push("application_name", "gpql");

    return out;
}

pub async fn open(config: &SessionConfig) -> Result<Session, String> {
    use crate::backends::Transport;

    return match crate::backends::transport_of(&config.kind) {
        Transport::Sqlite => open_sqlite(config),
        Transport::DuckDb => open_duck(config),
        Transport::Http => open_http(config),
        Transport::Redis => open_graph(config).await,
        Transport::MySql => open_mysql(config).await,
        Transport::Postgres => open_postgres(&flavoured(config)).await,
    };
}

fn open_duck(config: &SessionConfig) -> Result<Session, String> {
    let name = config
        .path
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("in memory")
        .to_string();

    return Ok(Session {
        engine: Engine::Duck(crate::duck::Duck::open(config)?),
        read_only: config.read_only,
        label: name,
        detail: config.path.clone(),
        kind: config.kind.clone(),
    });
}

async fn open_mysql(config: &SessionConfig) -> Result<Session, String> {
    let host = if config.host.is_empty() { "127.0.0.1" } else { &config.host };
    let port = if config.port.is_empty() { "3306" } else { &config.port };

    return Ok(Session {
        engine: Engine::MySql(crate::mysql::MySql::open(config).await?),
        read_only: config.read_only,
        label: config.database.clone(),
        detail: format!("{host}:{port}"),
        kind: config.kind.clone(),
    });
}

fn flavoured(config: &SessionConfig) -> SessionConfig {
    let mut out = config.clone();

    match config.kind.as_str() {
        "supabase" => {
            if !out.host.contains('.') && !out.host.is_empty() {
                out.host = format!("db.{}.supabase.co", out.host);
            }
            if out.port.is_empty() {
                out.port = "5432".into();
            }
            if out.user.is_empty() {
                out.user = "postgres".into();
            }
            if out.database.is_empty() {
                out.database = "postgres".into();
            }
            out.tls = "require".into();
        }
        "greptimedb" => {
            if out.port.is_empty() {
                out.port = "4003".into();
            }
            if out.database.is_empty() {
                out.database = "public".into();
            }
        }
        _ => {}
    }

    return out;
}

fn open_http(config: &SessionConfig) -> Result<Session, String> {
    if config.url.is_empty() {
        return Err("that connection needs a URL".into());
    }

    let label = if config.database.is_empty() {
        config.kind.clone()
    } else {
        config.database.clone()
    };

    return Ok(Session {
        engine: Engine::Http(crate::remote::Http::open(config)),
        read_only: config.read_only,
        label,
        detail: config.url.clone(),
        kind: config.kind.clone(),
    });
}

async fn open_graph(config: &SessionConfig) -> Result<Session, String> {
    let graph = crate::remote::Graph::open(config).await?;
    let label = graph.name.clone();

    return Ok(Session {
        engine: Engine::Graph(graph),
        read_only: config.read_only,
        label,
        detail: config.url.clone(),
        kind: config.kind.clone(),
    });
}

fn tls_connector() -> MakeRustlsConnect {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let settings = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    return MakeRustlsConnect::new(settings);
}

async fn open_postgres(config: &SessionConfig) -> Result<Session, String> {
    let wanted = if config.tls.is_empty() { "prefer" } else { &config.tls };
    let conn_string = pg_conn_string(config);

    let client = match wanted {
        "disable" => plain(&conn_string).await?,
        "require" => secure(&conn_string).await?,
        _ => match secure(&conn_string).await {
            Ok(client) => client,
            Err(_) => plain(&conn_string).await?,
        },
    };

    if config.read_only {
        client
            .batch_execute("set session characteristics as transaction read only")
            .await
            .map_err(friendly_pg)?;
    }

    let host = if config.host.is_empty() { "127.0.0.1" } else { &config.host };
    let port = if config.port.is_empty() { "5432" } else { &config.port };

    return Ok(Session {
        engine: Engine::Postgres(client),
        read_only: config.read_only,
        label: config.database.clone(),
        detail: format!("{host}:{port}"),
        kind: "postgres".into(),
    });
}

async fn secure(conn_string: &str) -> Result<tokio_postgres::Client, String> {
    let (client, connection) = tokio_postgres::connect(conn_string, tls_connector())
        .await
        .map_err(friendly_pg)?;

    tokio::spawn(async move {
        let _ = connection.await;
    });

    return Ok(client);
}

async fn plain(conn_string: &str) -> Result<tokio_postgres::Client, String> {
    let (client, connection) = tokio_postgres::connect(conn_string, NoTls)
        .await
        .map_err(friendly_pg)?;

    tokio::spawn(async move {
        let _ = connection.await;
    });

    return Ok(client);
}

fn open_sqlite(config: &SessionConfig) -> Result<Session, String> {
    let flags = if config.read_only {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    };

    let connection = Connection::open_with_flags(&config.path, flags).map_err(friendly)?;
    let name = config
        .path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&config.path)
        .to_string();

    return Ok(Session {
        engine: Engine::Sqlite(Mutex::new(connection)),
        read_only: config.read_only,
        label: name,
        detail: config.path.clone(),
        kind: "sqlite".into(),
    });
}

pub async fn query(session: &Session, sql: &str) -> Result<QueryResult, String> {
    match &session.engine {
        Engine::Postgres(client) => query_postgres(client, sql).await,
        Engine::Sqlite(connection) => query_sqlite(&connection.lock().unwrap(), sql),
        Engine::MySql(client) => client.query(sql).await,
        Engine::Duck(duck) => duck.query(sql),
        Engine::Http(remote) => remote.query(sql).await,
        Engine::Graph(graph) => graph.query(sql).await,
    }
}

async fn query_postgres(client: &tokio_postgres::Client, sql: &str) -> Result<QueryResult, String> {
    let messages = client.simple_query(sql).await.map_err(friendly_pg)?;

    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut affected = None;

    for message in messages {
        match message {
            SimpleQueryMessage::Row(row) => {
                if columns.is_empty() {
                    columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                }

                let values = (0..row.columns().len())
                    .map(|index| row.get(index).map(str::to_string))
                    .collect();

                rows.push(values);
            }
            SimpleQueryMessage::CommandComplete(count) => affected = Some(count),
            _ => {}
        }
    }

    return Ok(QueryResult { columns, rows, affected });
}

fn query_sqlite(connection: &Connection, sql: &str) -> Result<QueryResult, String> {
    let mut statement = connection.prepare(sql).map_err(friendly)?;
    let width = statement.column_count();

    if width == 0 {
        drop(statement);
        let affected = connection.execute(sql, []).map_err(friendly)?;

        return Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected: Some(affected as u64),
        });
    }

    let columns: Vec<String> = statement
        .column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();

    let mut rows = Vec::new();
    let mut cursor = statement.query([]).map_err(friendly)?;

    while let Some(row) = cursor.next().map_err(friendly)? {
        let mut values = Vec::with_capacity(width);

        for index in 0..width {
            values.push(cell_text(row.get_ref(index).map_err(friendly)?));
        }

        rows.push(values);
    }

    return Ok(QueryResult { columns, rows, affected: None });
}

fn cell_text(value: ValueRef<'_>) -> Option<String> {
    match value {
        ValueRef::Null => None,
        ValueRef::Integer(number) => Some(number.to_string()),
        ValueRef::Real(number) => Some(number.to_string()),
        ValueRef::Text(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => Some(format!("{} bytes", bytes.len())),
    }
}

pub async fn tables(session: &Session) -> Result<Vec<TableInfo>, String> {
    match &session.engine {
        Engine::Postgres(client) => {
            let result = query_postgres(
                client,
                "select c.relname, coalesce(s.n_live_tup, 0)::text
                 from pg_class c
                 join pg_namespace n on n.oid = c.relnamespace
                 left join pg_stat_user_tables s on s.relid = c.oid
                 where c.relkind = 'r' and n.nspname = 'public'
                 order by c.relname",
            )
            .await?;

            return Ok(result
                .rows
                .into_iter()
                .map(|row| TableInfo {
                    name: row[0].clone().unwrap_or_default(),
                    rows: row[1].as_deref().unwrap_or("0").parse().unwrap_or(0),
                })
                .collect());
        }
        Engine::Sqlite(connection) => {
            let connection = connection.lock().unwrap();
            let names = query_sqlite(
                &connection,
                "select name from sqlite_master
                 where type = 'table' and name not like 'sqlite_%'
                 order by name",
            )?;

            let mut out = Vec::new();

            for row in names.rows {
                let name = row[0].clone().unwrap_or_default();
                let counted = query_sqlite(
                    &connection,
                    &format!("select count(*) from {}", quote_ident(&name)),
                )?;
                let rows = counted.rows[0][0]
                    .as_deref()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);

                out.push(TableInfo { name, rows });
            }

            return Ok(out);
        }
        Engine::MySql(client) => client.tables().await,
        Engine::Duck(duck) => duck.tables(),
        Engine::Http(remote) => remote.tables().await,
        Engine::Graph(graph) => graph.tables().await,
    }
}

pub async fn table_rows(
    session: &Session,
    table: &str,
    limit: u32,
    offset: u32,
) -> Result<QueryResult, String> {
    let sql = match &session.engine {
        Engine::Graph(_) => format!("match (n:{table}) return n limit {limit}"),
        _ => format!(
            "select * from {} limit {limit} offset {offset}",
            quote_ident(table)
        ),
    };

    return query(session, &sql).await;
}

pub async fn schema(session: &Session) -> Result<Vec<TableSchema>, String> {
    let counts: HashMap<String, i64> = tables(session)
        .await?
        .into_iter()
        .map(|table| (table.name, table.rows))
        .collect();

    let mut out = match &session.engine {
        Engine::Postgres(client) => {
            let mut tables = postgres_schema(client).await?;
            let notes = postgres_notes(client).await.unwrap_or_default();

            for (table, column, raw) in notes {
                let (text, hints) = annotation(&raw);

                let Some(found) = tables.iter_mut().find(|entry| entry.name == table)
                else {
                    continue;
                };

                if column.is_empty() {
                    found.note = text.or(Some(raw));
                    found.hints = hints;
                    continue;
                }

                if let Some(target) =
                    found.columns.iter_mut().find(|entry| entry.name == column)
                {
                    target.note = text.or(Some(raw));
                    found.hints.extend(hints);
                }
            }

            tables
        }
        Engine::Sqlite(connection) => sqlite_schema(&connection.lock().unwrap(), &counts)?,
        Engine::MySql(client) => mysql_schema(client.columns().await?),
        Engine::Duck(duck) => mysql_schema(duck.columns()?),
        _ => counts
            .keys()
            .map(|name| TableSchema {
                name: name.clone(),
                rows: 0,
                columns: Vec::new(),
                note: None,
                hints: Vec::new(),
            })
            .collect(),
    };

    for table in &mut out {
        table.rows = *counts.get(&table.name).unwrap_or(&0);
    }

    return Ok(out);
}

async fn postgres_schema(client: &tokio_postgres::Client) -> Result<Vec<TableSchema>, String> {
    let columns = query_postgres(
        client,
        "select table_name, column_name, data_type, is_nullable
         from information_schema.columns
         where table_schema = 'public'
         order by table_name, ordinal_position",
    )
    .await?;

    let keys = query_postgres(
        client,
        "select tc.constraint_type, kcu.table_name, kcu.column_name,
                ccu.table_name as target_table, ccu.column_name as target_column
         from information_schema.table_constraints tc
         join information_schema.key_column_usage kcu
           on kcu.constraint_name = tc.constraint_name
          and kcu.table_schema = tc.table_schema
         left join information_schema.constraint_column_usage ccu
           on ccu.constraint_name = tc.constraint_name
          and ccu.table_schema = tc.table_schema
         where tc.table_schema = 'public'
           and tc.constraint_type in ('PRIMARY KEY', 'FOREIGN KEY')",
    )
    .await?;

    let mut primary = std::collections::HashSet::new();
    let mut foreign: HashMap<(String, String), String> = HashMap::new();

    for row in keys.rows {
        let kind = row[0].clone().unwrap_or_default();
        let table = row[1].clone().unwrap_or_default();
        let column = row[2].clone().unwrap_or_default();

        if kind == "PRIMARY KEY" {
            primary.insert((table, column));
            continue;
        }

        let target_table = row[3].clone().unwrap_or_default();
        let target_column = row[4].clone().unwrap_or_default();

        foreign.insert((table, column), format!("{target_table}.{target_column}"));
    }

    let mut grouped: Vec<TableSchema> = Vec::new();

    for row in columns.rows {
        let table = row[0].clone().unwrap_or_default();
        let name = row[1].clone().unwrap_or_default();
        let data_type = row[2].clone().unwrap_or_default();
        let required = row[3].as_deref() == Some("NO");

        if grouped.last().map(|last| last.name != table).unwrap_or(true) {
            grouped.push(TableSchema {
                name: table.clone(),
                rows: 0,
                columns: Vec::new(),
                note: None,
                hints: Vec::new(),
            });
        }

        grouped.last_mut().unwrap().columns.push(ColumnInfo {
            primary_key: primary.contains(&(table.clone(), name.clone())),
            references: foreign.get(&(table.clone(), name.clone())).cloned(),
            name,
            data_type,
            required,
            note: None,
        });
    }

    return Ok(grouped);
}

pub fn annotation(raw: &str) -> (Option<String>, Vec<String>) {
    let Some(start) = raw.find("@gpql:comment") else {
        return (None, Vec::new());
    };

    let rest = &raw[start + "@gpql:comment".len()..];
    let quoted = rest
        .split_once('"')
        .and_then(|(_, tail)| tail.split_once('"'))
        .map(|(body, _)| body.trim().to_string());

    let Some(text) = quoted else {
        return (None, Vec::new());
    };

    let hints = text
        .split_whitespace()
        .filter_map(|word| word.strip_prefix("@ref"))
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '.'))
        .filter(|word| word.contains('.'))
        .map(str::to_string)
        .collect::<Vec<_>>();

    let extra = text
        .split_whitespace()
        .enumerate()
        .filter(|(index, word)| {
            !word.starts_with("@ref")
                && !(*index > 0 && text.split_whitespace().nth(index - 1) == Some("@ref"))
        })
        .map(|(_, word)| word)
        .collect::<Vec<_>>()
        .join(" ");

    return (Some(extra.trim().to_string()), hints);
}

async fn postgres_notes(
    client: &tokio_postgres::Client,
) -> Result<Vec<(String, String, String)>, String> {
    let listing = query_postgres(
        client,
        "select c.relname, coalesce(a.attname, ''), d.description
         from pg_description d
         join pg_class c on c.oid = d.objoid
         join pg_namespace n on n.oid = c.relnamespace
         left join pg_attribute a
           on a.attrelid = c.oid and a.attnum = d.objsubid
         where n.nspname = 'public'",
    )
    .await?;

    return Ok(listing
        .rows
        .into_iter()
        .map(|row| {
            (
                row[0].clone().unwrap_or_default(),
                row[1].clone().unwrap_or_default(),
                row[2].clone().unwrap_or_default(),
            )
        })
        .collect());
}

fn mysql_schema(listing: QueryResult) -> Vec<TableSchema> {
    let mut grouped: Vec<TableSchema> = Vec::new();

    for row in listing.rows {
        let cell = |index: usize| row.get(index).cloned().flatten().unwrap_or_default();
        let table = cell(0);

        if grouped.last().map(|last| last.name != table).unwrap_or(true) {
            grouped.push(TableSchema {
                name: table.clone(),
                rows: 0,
                columns: Vec::new(),
                note: None,
                hints: Vec::new(),
            });
        }

        let target = cell(5);

        grouped.last_mut().unwrap().columns.push(ColumnInfo {
            name: cell(1),
            data_type: cell(2),
            required: cell(3) == "NO",
            primary_key: cell(4) == "PRI",
            note: None,
            references: if target.is_empty() {
                None
            } else {
                Some(format!("{target}.{}", cell(6)))
            },
        });
    }

    return grouped;
}

fn sqlite_schema(
    connection: &Connection,
    counts: &HashMap<String, i64>,
) -> Result<Vec<TableSchema>, String> {
    let mut names: Vec<&String> = counts.keys().collect();
    names.sort();

    let mut out = Vec::new();

    for name in names {
        let info = query_sqlite(
            connection,
            &format!("pragma table_info({})", quote_ident(name)),
        )?;
        let links = query_sqlite(
            connection,
            &format!("pragma foreign_key_list({})", quote_ident(name)),
        )?;

        let mut foreign: HashMap<String, String> = HashMap::new();

        for row in links.rows {
            let from = row[3].clone().unwrap_or_default();
            let target_table = row[2].clone().unwrap_or_default();
            let target_column = row[4].clone().unwrap_or_else(|| "rowid".into());

            foreign.insert(from, format!("{target_table}.{target_column}"));
        }

        let columns = info
            .rows
            .into_iter()
            .map(|row| {
                let column = row[1].clone().unwrap_or_default();

                ColumnInfo {
                    data_type: row[2].clone().unwrap_or_default().to_lowercase(),
                    required: row[3].as_deref() == Some("1"),
                    primary_key: row[5].as_deref().unwrap_or("0") != "0",
                    references: foreign.get(&column).cloned(),
                    note: None,
                    name: column,
                }
            })
            .collect();

        out.push(TableSchema {
            name: name.clone(),
            rows: 0,
            columns,
            note: None,
            hints: Vec::new(),
        });
    }

    return Ok(out);
}

pub fn local_postgres_ports() -> Vec<u16> {
    return (5432..=5435).filter(|port| reachable("127.0.0.1", *port, 120)).collect();
}

pub fn reachable(host: &str, port: u16, patience: u64) -> bool {
    use std::net::ToSocketAddrs;

    let Ok(mut addresses) = (host, port).to_socket_addrs() else {
        return false;
    };

    return addresses
        .any(|address| TcpStream::connect_timeout(&address, Duration::from_millis(patience)).is_ok());
}

fn friendly(error: impl std::fmt::Display) -> String {
    let text = error.to_string();

    return text
        .strip_prefix("error connecting to server: ")
        .unwrap_or(&text)
        .to_string();
}

fn friendly_pg(error: tokio_postgres::Error) -> String {
    if let Some(reported) = error.as_db_error() {
        return reported.message().to_string();
    }

    let mut text = error.to_string();
    let mut cause = std::error::Error::source(&error);

    while let Some(inner) = cause {
        text = format!("{text}: {inner}");
        cause = inner.source();
    }

    return plain_message(&text);
}

fn plain_message(text: &str) -> String {
    let known = [
        ("password missing", "this server wants a password"),
        ("os error 10061", "nothing is listening there"),
        ("Connection refused", "nothing is listening there"),
        ("os error 10060", "the host never answered"),
        ("failed to lookup address", "that host name goes nowhere"),
    ];

    for (needle, friendly) in known {
        if text.contains(needle) {
            return friendly.to_string();
        }
    }

    return text
        .strip_prefix("error connecting to server: ")
        .unwrap_or(text)
        .to_string();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub needs_login: bool,
}

pub async fn scan_host(
    host: &str,
    ports: &[u16],
    candidates: &[(String, String)],
) -> Vec<Discovery> {
    let mut found = Vec::new();

    for port in ports.iter().copied().filter(|port| reachable(host, *port, 200)) {
        let mut reached = false;

        for (user, password) in candidates {
            let probe = SessionConfig {
                kind: "postgres".into(),
                host: host.to_string(),
                port: port.to_string(),
                user: user.clone(),
                password: password.clone(),
                database: "postgres".into(),
                path: String::new(),
                read_only: true,
                tls: "prefer".into(),
                url: String::new(),
                token: String::new(),
            };

            let Ok(session) = open(&probe).await else {
                continue;
            };
            let Ok(result) = query(
                &session,
                "select datname from pg_database
                 where datistemplate = false and datallowconn = true
                 order by datname",
            )
            .await
            else {
                continue;
            };

            reached = true;

            for row in result.rows {
                found.push(Discovery {
                    host: host.to_string(),
                    port: port.to_string(),
                    user: user.clone(),
                    password: password.clone(),
                    database: row[0].clone().unwrap_or_default(),
                    needs_login: false,
                });
            }

            break;
        }

        if !reached {
            found.push(Discovery {
                host: host.to_string(),
                port: port.to_string(),
                user: candidates
                    .first()
                    .map(|(user, _)| user.clone())
                    .unwrap_or_else(|| "postgres".into()),
                password: String::new(),
                database: String::new(),
                needs_login: true,
            });
        }
    }

    return found;
}

pub async fn scan(candidates: &[(String, String)]) -> Vec<Discovery> {
    return scan_host("127.0.0.1", &[5432, 5433, 5434, 5435], candidates).await;
}
