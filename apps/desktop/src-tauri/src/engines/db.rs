use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use tokio_postgres::{NoTls, SimpleQueryMessage};

use super::errors::{friendly, friendly_pg};
use super::slicing::sliceable;
use super::writing::{begin_if_manual, transactional};
use tokio_postgres_rustls::MakeRustlsConnect;

#[derive(Clone, Deserialize, Default)]
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
    #[serde(default)]
    pub warehouse: String,
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub tunnel: crate::net::tunnel::TunnelConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHandle {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub kind: String,
    pub read_only: bool,
    pub sliceable: bool,
    pub transactional: bool,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableSchema {
    pub name: String,
    pub rows: i64,
    pub columns: Vec<ColumnInfo>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub policies: Vec<String>,
}

pub enum Engine {
    Postgres(tokio_postgres::Client),
    MySql(crate::engines::mysql::MySql),
    Duck(crate::engines::duck::Duck),
    Sqlite(Mutex<Connection>),
    Http(crate::engines::remote::Http),
    Driver(Box<crate::engines::drivers::Driver>),
    Graph(crate::engines::remote::Graph),
}

pub struct Session {
    pub engine: Engine,
    pub read_only: AtomicBool,
    pub manual: AtomicBool,
    pub open_tx: AtomicBool,
    pub label: String,
    pub detail: String,
    pub kind: String,
}

#[derive(Default)]
pub struct Sessions {
    open: Mutex<HashMap<String, Arc<Session>>>,
    next: AtomicU64,
}

impl Session {
    pub fn set_read_only(&self, on: bool) {
        self.read_only.store(on, Ordering::Relaxed);
    }
}

impl Sessions {
    pub fn insert(&self, session: Session) -> SessionHandle {
        let id = format!("s{}", self.next.fetch_add(1, Ordering::Relaxed));
        let handle = SessionHandle {
            id: id.clone(),
            label: session.label.clone(),
            detail: session.detail.clone(),
            kind: session.kind.clone(),
            read_only: session.read_only.load(Ordering::Relaxed),
            sliceable: sliceable(&session),
            transactional: transactional(&session),
        };

        self.open.lock().unwrap().insert(id, Arc::new(session));

        return handle;
    }

    // a webview reload drops every handle the frontend held, so the sessions
    // behind them leak server slots unless the fresh page clears them
    pub fn clear(&self) {
        self.open.lock().unwrap().clear();
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

pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub fn quote_for(session: &Session, name: &str) -> String {
    return match &session.engine {
        Engine::MySql(_) => format!("`{}`", name.replace('`', "``")),
        // d1 is sqlite and takes either, but supabase is postgres over http
        // and rejects a backtick outright
        Engine::Http(remote) if remote.flavour != "supabase_api" => {
            format!("`{}`", name.replace('`', "``"))
        }
        _ => quote_ident(name),
    };
}

pub fn literal(value: &Option<String>) -> String {
    return match value {
        None => "null".to_string(),
        Some(text) => format!("'{}'", text.replace('\'', "''")),
    };
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
    use crate::engines::backends::Transport;

    return match crate::engines::backends::transport_of(&config.kind) {
        Transport::Driver => open_driver(config).await,
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
        engine: Engine::Duck(crate::engines::duck::Duck::open(config)?),
        read_only: AtomicBool::new(config.read_only),
        manual: AtomicBool::new(false),
        open_tx: AtomicBool::new(false),
        label: name,
        detail: config.path.clone(),
        kind: config.kind.clone(),
    });
}

async fn open_mysql(config: &SessionConfig) -> Result<Session, String> {
    let host = if config.host.is_empty() { "127.0.0.1" } else { &config.host };
    let port = if config.port.is_empty() { "3306" } else { &config.port };

    return Ok(Session {
        engine: Engine::MySql(crate::engines::mysql::MySql::open(config).await?),
        read_only: AtomicBool::new(config.read_only),
        manual: AtomicBool::new(false),
        open_tx: AtomicBool::new(false),
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
            if out.tls.is_empty() {
                out.tls = "require".into();
            }
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

async fn open_driver(config: &SessionConfig) -> Result<Session, String> {
    let driver = crate::engines::drivers::Driver::open(config).await?;

    let label = if config.database.is_empty() {
        config.kind.clone()
    } else {
        config.database.clone()
    };

    let detail = if config.url.is_empty() {
        config.host.clone()
    } else {
        config.url.clone()
    };

    return Ok(Session {
        engine: Engine::Driver(Box::new(driver)),
        read_only: AtomicBool::new(config.read_only),
        manual: AtomicBool::new(false),
        open_tx: AtomicBool::new(false),
        label,
        detail,
        kind: config.kind.clone(),
    });
}

fn open_http(config: &SessionConfig) -> Result<Session, String> {
    let hosted = config.kind == "supabase_api";

    if config.url.is_empty() && !hosted {
        return Err("that connection needs a URL".into());
    }

    if hosted && config.database.is_empty() {
        return Err("that connection needs a project".into());
    }

    let label = if config.database.is_empty() {
        config.kind.clone()
    } else {
        config.database.clone()
    };

    let detail = if hosted {
        "supabase".to_string()
    } else {
        config.url.clone()
    };

    return Ok(Session {
        engine: Engine::Http(crate::engines::remote::Http::open(config)),
        read_only: AtomicBool::new(config.read_only),
        manual: AtomicBool::new(false),
        open_tx: AtomicBool::new(false),
        label,
        detail,
        kind: config.kind.clone(),
    });
}

async fn open_graph(config: &SessionConfig) -> Result<Session, String> {
    let graph = crate::engines::remote::Graph::open(config).await?;
    let label = graph.name.clone();

    return Ok(Session {
        engine: Engine::Graph(graph),
        read_only: AtomicBool::new(config.read_only),
        manual: AtomicBool::new(false),
        open_tx: AtomicBool::new(false),
        label,
        detail: config.url.clone(),
        kind: config.kind.clone(),
    });
}

// reqwest pulls rustls/ring and clickhouse pulls rustls/aws-lc-rs, so rustls
// sees two providers and refuses to pick one; without this every builder call
// below panics instead of connecting
fn install_crypto() {
    static ONCE: std::sync::Once = std::sync::Once::new();

    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn tls_connector(verify: bool) -> MakeRustlsConnect {
    install_crypto();

    let settings = if verify {
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth()
    } else {
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(SkipChainCheck))
            .with_no_client_auth()
    };

    return MakeRustlsConnect::new(settings);
}

// libpq sslmode=require encrypts without checking the chain; hosts behind a
// private CA only work under those rules. verify-full keeps the real check.
#[derive(Debug)]
struct SkipChainCheck;

impl rustls::client::danger::ServerCertVerifier for SkipChainCheck {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        return Ok(rustls::client::danger::ServerCertVerified::assertion());
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        return Ok(rustls::client::danger::HandshakeSignatureValid::assertion());
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        return Ok(rustls::client::danger::HandshakeSignatureValid::assertion());
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        return rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes();
    }
}

async fn open_postgres(config: &SessionConfig) -> Result<Session, String> {
    let wanted = if config.tls.is_empty() { "prefer" } else { &config.tls };
    let conn_string = pg_conn_string(config);

    let client = match wanted {
        "disable" => plain(&conn_string).await?,
        "verify-full" => secure(&conn_string, true).await?,
        "require" => secure(&conn_string, false).await?,
        _ => match secure(&conn_string, false).await {
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
        read_only: AtomicBool::new(config.read_only),
        manual: AtomicBool::new(false),
        open_tx: AtomicBool::new(false),
        label: config.database.clone(),
        detail: format!("{host}:{port}"),
        kind: "postgres".into(),
    });
}

async fn secure(
    conn_string: &str,
    verify: bool,
) -> Result<tokio_postgres::Client, String> {
    let (client, connection) = tokio_postgres::connect(conn_string, tls_connector(verify))
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
        read_only: AtomicBool::new(config.read_only),
        manual: AtomicBool::new(false),
        open_tx: AtomicBool::new(false),
        label: name,
        detail: config.path.clone(),
        kind: "sqlite".into(),
    });
}

#[cfg(test)]
mod refusing {
    use super::reads_only;

    #[test]
    fn lets_reads_through() {
        assert!(reads_only("select 1"));
        assert!(reads_only("  SELECT * from t"));
        assert!(reads_only("from(bucket: \"b\") |> range(start: 0)"));
        assert!(reads_only("MATCH (n) RETURN n"));
        assert!(reads_only("explain select 1"));
        assert!(reads_only("show tables"));
    }

    #[test]
    fn stops_writes() {
        assert!(!reads_only("delete from t"));
        assert!(!reads_only("  DROP TABLE t"));
        assert!(!reads_only("insert into t values (1)"));
        assert!(!reads_only("update t set a = 1"));
        assert!(!reads_only("CREATE (n:Node)"));
        assert!(!reads_only("merge into t"));
    }
}

// the ui promises read only, so every engine refuses writes here, not just the
// ones whose server can be asked to
pub fn reads_only(sql: &str) -> bool {
    // flux writes `from(bucket:)` with no space, so cut at the first
    // non-letter instead of at whitespace
    let head: String = sql
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase();

    return matches!(
        head.as_str(),
        "select"
            | "show"
            | "describe"
            | "desc"
            | "explain"
            | "with"
            | "pragma"
            | "from"
            | "match"
            | "return"
            | "unwind"
            | "values"
            | "table"
    );
}

pub async fn query(session: &Session, sql: &str) -> Result<QueryResult, String> {
    let writes = !reads_only(sql);

    if session.read_only.load(Ordering::Relaxed) && writes {
        return Err("this session is read only".into());
    }

    if writes {
        begin_if_manual(session).await?;
    }

    match &session.engine {
        Engine::Postgres(client) => query_postgres(client, sql).await,
        Engine::Sqlite(connection) => query_sqlite(&connection.lock().unwrap(), sql),
        Engine::MySql(client) => client.query(sql).await,
        Engine::Duck(duck) => duck.query(sql),
        Engine::Http(remote) => remote.query(sql).await,
        Engine::Driver(driver) => driver.query(sql).await,
        Engine::Graph(graph) => graph.query(sql).await,
    }
}

pub async fn query_postgres(client: &tokio_postgres::Client, sql: &str) -> Result<QueryResult, String> {
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

pub fn query_sqlite(connection: &Connection, sql: &str) -> Result<QueryResult, String> {
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


#[cfg(test)]
#[path = "db_tests.rs"]
mod db_tests;
