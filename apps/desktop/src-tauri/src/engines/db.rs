use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use tokio_postgres::config::SslMode;
use tokio_postgres::{NoTls, SimpleQueryMessage};

use super::access::{access, calls_routine, ending, keeps_transaction, plain_read, Access, Ending};
use super::errors::{friendly, friendly_pg};
use super::slicing::sliceable;
use super::writing::{begin_if_manual, finish, note_failure, transactional};
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
    #[serde(default)]
    pub dial: String,
    #[serde(default)]
    pub create: bool,
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

impl QueryResult {
    pub fn empty() -> Self {
        QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected: None,
        }
    }
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
    Sqlite(Arc<Mutex<Connection>>),
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
    pub flavour: String,
    pub(crate) server_guard: AtomicBool,
    pub(crate) tls_verify: bool,
    pub(crate) tx_lock: tokio::sync::Mutex<()>,
    pub(crate) tx_failure: Mutex<Option<String>>,
    pub(crate) search_path: Mutex<Option<String>>,
    pub(crate) order_keys: Mutex<HashMap<String, Vec<String>>>,
    pub(crate) interrupt: Mutex<Option<rusqlite::InterruptHandle>>,
}

#[derive(Default)]
pub struct Sessions {
    open: Mutex<HashMap<String, Arc<Session>>>,
    next: AtomicU64,
}

pub(crate) fn held<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Session {
    fn new(
        engine: Engine,
        config: &SessionConfig,
        label: String,
        detail: String,
        kind: &str,
    ) -> Self {
        Session {
            engine,
            read_only: AtomicBool::new(config.read_only),
            manual: AtomicBool::new(false),
            open_tx: AtomicBool::new(false),
            label,
            detail,
            kind: kind.to_string(),
            flavour: config.kind.clone(),
            server_guard: AtomicBool::new(config.read_only),
            tls_verify: false,
            tx_lock: tokio::sync::Mutex::new(()),
            tx_failure: Mutex::new(None),
            search_path: Mutex::new(None),
            order_keys: Mutex::new(HashMap::new()),
            interrupt: Mutex::new(None),
        }
    }

    pub fn set_read_only(&self, on: bool) {
        self.read_only.store(on, Ordering::Relaxed);
    }

    pub fn on_catalog_change(&self, notify: impl Fn(&str) + Send + 'static) {
        if let Engine::Driver(driver) = &self.engine {
            driver.on_catalog_change(Box::new(notify));
        }
    }

    pub fn s3(&self) -> Result<&crate::engines::s3::S3, String> {
        match &self.engine {
            Engine::Driver(driver) => driver.s3(),
            _ => Err("not an s3 session".to_string()),
        }
    }

    pub fn mqtt(&self) -> Result<&crate::engines::mqtt::Mqtt, String> {
        match &self.engine {
            Engine::Driver(driver) => driver.mqtt(),
            _ => Err("not an mqtt session".to_string()),
        }
    }

    pub async fn cancel(&self) -> Result<(), String> {
        match &self.engine {
            Engine::Postgres(client) => client
                .cancel_token()
                .cancel_query(MakeRustlsConnect::new(tls_config(self.tls_verify)))
                .await
                .map_err(friendly_pg),
            Engine::MySql(client) => client.cancel().await,
            Engine::Sqlite(_) => match held(&self.interrupt).as_ref() {
                Some(handle) => {
                    handle.interrupt();

                    Ok(())
                }
                None => Err("this session can not stop a running query".into()),
            },
            Engine::Duck(duck) => {
                duck.cancel();

                Ok(())
            }
            Engine::Driver(driver) => driver.cancel().await,
            _ => Err("this engine can not stop a running query".into()),
        }
    }

    // greptime only fakes transactions over the postgres wire, so it enforces nothing
    pub fn guarded(&self) -> bool {
        let enforced = match &self.engine {
            Engine::Postgres(_) => self.flavour != "greptimedb",
            Engine::MySql(_) | Engine::Graph(_) => true,
            Engine::Sqlite(_) => !sqlite_in_memory(&self.detail),
            Engine::Duck(duck) => !duck.in_memory(),
            Engine::Http(_) => false,
            Engine::Driver(driver) => driver.guarded(),
        };

        enforced && self.server_guard.load(Ordering::Relaxed)
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

        held(&self.open).insert(id, Arc::new(session));

        handle
    }

    // a webview reload drops every handle the frontend held, so the sessions
    // behind them leak server slots unless the fresh page clears them
    pub fn clear(&self) {
        held(&self.open).clear();
    }

    pub fn get(&self, id: &str) -> Result<Arc<Session>, String> {
        held(&self.open)
            .get(id)
            .cloned()
            .ok_or_else(|| "that session is closed".to_string())
    }

    pub fn remove(&self, id: &str) {
        held(&self.open).remove(id);
    }
}

pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub fn quote_for(session: &Session, name: &str) -> String {
    match &session.engine {
        Engine::MySql(_) => format!("`{}`", name.replace('`', "``")),
        // d1 is sqlite and takes either, but supabase is postgres over http
        // and rejects a backtick outright
        Engine::Http(remote) if remote.flavour != "supabase_api" => {
            format!("`{}`", name.replace('`', "``"))
        }
        _ => quote_ident(name),
    }
}

pub fn literal(flavour: &str, value: &Option<String>) -> String {
    let Some(text) = value else {
        return "null".to_string();
    };

    match flavour {
        "mysql" | "clickhouse" | "snowflake" => {
            format!("'{}'", text.replace('\\', "\\\\").replace('\'', "''"))
        }
        // an E'' string reads backslashes the same way whatever
        // standard_conforming_strings says
        "postgres" | "supabase" | "supabase_api" if text.contains('\\') => {
            format!("E'{}'", text.replace('\\', "\\\\").replace('\'', "''"))
        }
        _ => format!("'{}'", text.replace('\'', "''")),
    }
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut out = String::with_capacity(2 + bytes.len() * 2);

    out.push_str("0x");

    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }

    out
}

pub(crate) fn port_or(text: &str, fallback: u16) -> Result<u16, String> {
    let text = text.trim();

    if text.is_empty() {
        return Ok(fallback);
    }

    text.parse()
        .map_err(|_| format!("{text} is not a port number"))
}

fn secret(key: &str) -> bool {
    let key = key.to_ascii_lowercase();

    [
        "token",
        "pass",
        "pwd",
        "secret",
        "key",
        "auth",
        "sig",
        "credential",
    ]
    .iter()
    .any(|word| key.contains(word))
}

// the detail is shown on screen and kept with the session, so a password or
// token riding in the url stays behind
fn shown(address: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(address) else {
        return address.to_string();
    };

    let kept: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(key, _)| !secret(key))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    let dropped = parsed.query_pairs().count() != kept.len();

    if parsed.password().is_none() && !dropped {
        return address.to_string();
    }

    let _ = parsed.set_password(None);

    if kept.is_empty() {
        parsed.set_query(None);
    } else {
        parsed.query_pairs_mut().clear().extend_pairs(kept);
    }

    parsed.to_string()
}

fn or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

fn pg_config(config: &SessionConfig) -> Result<tokio_postgres::Config, String> {
    let mut pg = tokio_postgres::Config::new();

    pg.host(or(&config.host, "127.0.0.1"));
    pg.port(port_or(&config.port, 5432)?);
    pg.application_name("gpql");

    if !config.dial.is_empty() {
        let address = config
            .dial
            .parse()
            .map_err(|_| format!("the tunnel address {} is not an IP address", config.dial))?;

        pg.hostaddr(address);
    }

    if !config.user.is_empty() {
        pg.user(&config.user);
    }

    if !config.password.is_empty() {
        pg.password(&config.password);
    }

    if !config.database.is_empty() {
        pg.dbname(&config.database);
    }

    Ok(pg)
}

pub async fn open(config: &SessionConfig) -> Result<Session, String> {
    use crate::engines::backends::Transport;

    match crate::engines::backends::transport_of(&config.kind) {
        Transport::Driver => open_driver(config).await,
        Transport::Sqlite => open_sqlite(config).await,
        Transport::DuckDb => open_duck(config).await,
        Transport::Http => open_http(config),
        Transport::Redis => open_graph(config).await,
        Transport::MySql => open_mysql(config).await,
        Transport::Postgres => open_postgres(&flavoured(config)).await,
    }
}

async fn open_duck(config: &SessionConfig) -> Result<Session, String> {
    let name = config
        .path
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("in memory")
        .to_string();
    let duck = crate::engines::duck::Duck::open(config).await?;

    Ok(Session::new(
        Engine::Duck(duck),
        config,
        name,
        config.path.clone(),
        &config.kind,
    ))
}

async fn open_mysql(config: &SessionConfig) -> Result<Session, String> {
    let host = or(&config.host, "127.0.0.1");
    let port = port_or(&config.port, 3306)?;
    let client = crate::engines::mysql::MySql::open(config).await?;

    Ok(Session::new(
        Engine::MySql(client),
        config,
        config.database.clone(),
        format!("{host}:{port}"),
        &config.kind,
    ))
}

pub fn flavoured(config: &SessionConfig) -> SessionConfig {
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

    out
}

async fn open_driver(config: &SessionConfig) -> Result<Session, String> {
    let driver = crate::engines::drivers::Driver::open(config).await?;

    let label = if config.database.is_empty() {
        config.kind.clone()
    } else {
        config.database.clone()
    };

    let detail = if !config.url.is_empty() {
        shown(&config.url)
    } else if config.port.is_empty() {
        config.host.clone()
    } else {
        format!("{}:{}", config.host, config.port)
    };

    Ok(Session::new(
        Engine::Driver(Box::new(driver)),
        config,
        label,
        detail,
        &config.kind,
    ))
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
        shown(&config.url)
    };

    Ok(Session::new(
        Engine::Http(crate::engines::remote::Http::open(config)),
        config,
        label,
        detail,
        &config.kind,
    ))
}

async fn open_graph(config: &SessionConfig) -> Result<Session, String> {
    let graph = crate::engines::remote::Graph::open(config).await?;
    let label = graph.name.clone();

    Ok(Session::new(
        Engine::Graph(graph),
        config,
        label,
        shown(&config.url),
        &config.kind,
    ))
}

// reqwest pulls rustls/ring and clickhouse pulls rustls/aws-lc-rs, so rustls
// sees two providers and refuses to pick one; without this every builder call
// below panics instead of connecting
pub(crate) fn install_crypto() {
    static ONCE: std::sync::Once = std::sync::Once::new();

    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

pub(crate) fn tls_config(verify: bool) -> rustls::ClientConfig {
    install_crypto();

    if verify {
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        return rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
    }

    rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(SkipChainCheck))
        .with_no_client_auth()
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
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

// prefer asks for TLS and only goes on in plaintext when the server itself
// answers that it has none; any later failure ends the attempt, so a password
// is never resent unencrypted
async fn open_postgres(config: &SessionConfig) -> Result<Session, String> {
    let mut pg = pg_config(config)?;

    let (mode, verify) = match config.tls.as_str() {
        "disable" => (SslMode::Disable, false),
        "" | "prefer" => (SslMode::Prefer, false),
        "require" => (SslMode::Require, false),
        "verify-full" => (SslMode::Require, true),
        other => return Err(format!("{other} is not a TLS mode GPQL knows")),
    };

    pg.ssl_mode(mode);

    let client = if mode == SslMode::Disable {
        let (client, connection) = pg.connect(NoTls).await.map_err(friendly_pg)?;

        tokio::spawn(connection);

        client
    } else {
        let tls = MakeRustlsConnect::new(tls_config(verify));
        let (client, connection) = pg.connect(tls).await.map_err(friendly_pg)?;

        tokio::spawn(connection);

        client
    };

    if config.read_only && config.kind != "greptimedb" {
        client
            .batch_execute("set session characteristics as transaction read only")
            .await
            .map_err(friendly_pg)?;
    }

    let host = or(&config.host, "127.0.0.1");
    let port = or(&config.port, "5432");

    let mut session = Session::new(
        Engine::Postgres(client),
        config,
        config.database.clone(),
        format!("{host}:{port}"),
        "postgres",
    );

    session.tls_verify = verify;

    Ok(session)
}

pub(crate) fn sqlite_in_memory(path: &str) -> bool {
    path.is_empty() || path == ":memory:"
}

// read only lives in the file's open mode, which no typed pragma can lift
fn connect_sqlite(path: &str, read_only: bool, create: bool) -> Result<Connection, String> {
    let exists = std::path::Path::new(path).exists();

    if !create && path != ":memory:" && !exists {
        return Err(format!("{path} does not exist"));
    }

    if read_only && !sqlite_in_memory(path) {
        if !exists {
            Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
            )
            .map_err(friendly)?;
        }

        return Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(friendly);
    }

    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE;

    if create {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }

    let connection = Connection::open_with_flags(path, flags).map_err(friendly)?;

    if read_only {
        connection
            .execute_batch("pragma query_only = 1")
            .map_err(friendly)?;
    }

    Ok(connection)
}

async fn open_sqlite(config: &SessionConfig) -> Result<Session, String> {
    let path = config.path.clone();
    let create = config.create;
    let read_only = config.read_only;

    let (connection, interrupt) = tokio::task::spawn_blocking(move || {
        let connection = connect_sqlite(&path, read_only, create)?;
        let interrupt = connection.get_interrupt_handle();

        Ok::<_, String>((connection, interrupt))
    })
    .await
    .map_err(|error| error.to_string())??;

    let name = config
        .path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&config.path)
        .to_string();

    let session = Session::new(
        Engine::Sqlite(Arc::new(Mutex::new(connection))),
        config,
        name,
        config.path.clone(),
        "sqlite",
    );

    *held(&session.interrupt) = Some(interrupt);

    Ok(session)
}

pub(crate) async fn reopen_sqlite(session: &Session, read_only: bool) -> Result<(), String> {
    let Engine::Sqlite(connection) = &session.engine else {
        return Ok(());
    };

    let shared = Arc::clone(connection);
    let path = session.detail.clone();

    let interrupt = tokio::task::spawn_blocking(move || {
        let mut current = held(&shared);

        if !current.is_autocommit() {
            return Err(
                "commit or roll back the open transaction before switching read only".to_string(),
            );
        }

        if sqlite_in_memory(&path) {
            current
                .execute_batch(&format!("pragma query_only = {}", u8::from(read_only)))
                .map_err(friendly)?;
        } else {
            *current = connect_sqlite(&path, read_only, false)?;
        }

        Ok(current.get_interrupt_handle())
    })
    .await
    .map_err(|error| error.to_string())??;

    *held(&session.interrupt) = Some(interrupt);

    Ok(())
}

pub(crate) async fn with_sqlite<T: Send + 'static>(
    connection: &Arc<Mutex<Connection>>,
    work: impl FnOnce(&Connection) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let shared = Arc::clone(connection);

    tokio::task::spawn_blocking(move || work(&held(&shared)))
        .await
        .map_err(|error| error.to_string())?
}

pub async fn query(session: &Session, sql: &str) -> Result<QueryResult, String> {
    let kind = access(&session.flavour, sql);
    let read_only = session.read_only.load(Ordering::Relaxed);

    if read_only && kind == Access::Write {
        return Err("this session is read only".into());
    }

    if read_only
        && kind == Access::Unknown
        && !(session.guarded() && plain_read(&session.flavour, sql))
    {
        return Err(
            "this session is read only, and GPQL can not tell whether that statement writes; allow writes to run it"
                .into(),
        );
    }

    let transactional = transactional(session);
    let manual = session.manual.load(Ordering::Relaxed) && transactional;
    let control = if transactional {
        ending(&session.flavour, sql)
    } else {
        Ending::None
    };

    match control {
        Ending::None => {}
        _ if !manual => {
            return Err(
                "this session commits on its own; turn on manual commit to run begin, commit, rollback or savepoint yourself"
                    .into(),
            );
        }
        Ending::Commit => {
            return finish(session, "commit")
                .await
                .map(|_| QueryResult::empty())
        }
        Ending::Rollback => {
            return finish(session, "rollback")
                .await
                .map(|_| QueryResult::empty())
        }
        Ending::Mixed => {
            return Err(
                "this session commits by hand; end its transaction with commit or rollback on their own"
                    .into(),
            );
        }
        Ending::Savepoint => {}
    }

    if manual && matches!(session.engine, Engine::MySql(_)) {
        if !keeps_transaction(&session.flavour, sql) {
            return outside_transaction(session, sql).await;
        }

        // a routine that runs ddl commits the open transaction from inside its body
        if calls_routine(&session.flavour, sql) && session.open_tx.load(Ordering::Relaxed) {
            return Err(IMPLICIT_COMMIT.into());
        }
    }

    if kind != Access::Read || control == Ending::Savepoint {
        begin_if_manual(session).await?;
        held(&session.order_keys).clear();
    }

    read(session, sql).await
}

const IMPLICIT_COMMIT: &str = "mysql commits the open transaction by itself before this statement; commit or roll back first, then run it";

async fn outside_transaction(session: &Session, sql: &str) -> Result<QueryResult, String> {
    let _guard = session.tx_lock.lock().await;

    if session.open_tx.load(Ordering::Relaxed) {
        return Err(IMPLICIT_COMMIT.into());
    }

    held(&session.order_keys).clear();

    read(session, sql).await
}

// catalog lookups GPQL writes itself skip the classifier, but a failure still
// has to mark an open transaction as broken
pub(crate) async fn read(session: &Session, sql: &str) -> Result<QueryResult, String> {
    let read_only = session.read_only.load(Ordering::Relaxed);

    let result = match &session.engine {
        Engine::Postgres(client) => query_postgres(client, sql).await,
        Engine::Sqlite(connection) => {
            let sql = sql.to_string();

            with_sqlite(connection, move |connection| query_sqlite(connection, &sql)).await
        }
        Engine::MySql(client) => client.query(sql).await,
        Engine::Duck(duck) => duck.query(sql).await,
        Engine::Http(remote) => remote.query(sql, read_only).await,
        Engine::Driver(driver) => driver.query(sql, read_only).await,
        Engine::Graph(graph) => graph.query(sql, read_only).await,
    };

    if let Err(failure) = &result {
        note_failure(session, failure).await;
    }

    result
}

pub async fn query_postgres(
    client: &tokio_postgres::Client,
    sql: &str,
) -> Result<QueryResult, String> {
    let messages = client.simple_query(sql).await.map_err(friendly_pg)?;

    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut affected = None;

    for message in messages {
        match message {
            SimpleQueryMessage::RowDescription(described) => {
                columns = described.iter().map(|c| c.name().to_string()).collect();
                rows = Vec::new();
            }
            SimpleQueryMessage::Row(row) => {
                let values = (0..row.columns().len())
                    .map(|index| row.get(index).map(str::to_string))
                    .collect();

                rows.push(values);
            }
            SimpleQueryMessage::CommandComplete(count) => affected = Some(count),
            _ => {}
        }
    }

    Ok(QueryResult {
        columns,
        rows,
        affected,
    })
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

    Ok(QueryResult {
        columns,
        rows,
        affected: None,
    })
}

fn cell_text(value: ValueRef<'_>) -> Option<String> {
    match value {
        ValueRef::Null => None,
        ValueRef::Integer(number) => Some(number.to_string()),
        ValueRef::Real(number) => Some(number.to_string()),
        ValueRef::Text(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => Some(hex(bytes)),
    }
}

#[cfg(test)]
mod refusing {
    use crate::engines::access::{
        access, calls_routine, ending, keeps_transaction, plain_read, single_read, Access, Ending,
    };
    use crate::engines::db::{open, query, read, Session, SessionConfig};
    use crate::engines::writing::{finish, set_manual, set_read_only};

    fn reads(kind: &str, sql: &str) -> bool {
        access(kind, sql) == Access::Read
    }

    #[test]
    fn lets_reads_through() {
        assert!(reads("postgres", "select 1"));
        assert!(reads("postgres", "  SELECT * from t"));
        assert!(reads("influxdb2", "from(bucket: \"b\") |> range(start: 0)"));
        assert!(reads("neo4j", "MATCH (n) RETURN n"));
        assert!(reads("postgres", "explain select 1"));
        assert!(reads("mysql", "show tables"));
        assert!(reads("postgres", "explain delete from t"));
        assert!(reads("sqlite", "pragma table_info('t')"));
        assert!(reads("postgres", "set search_path to public"));
        assert!(reads("s3", "ls bucket"));
    }

    #[test]
    fn stops_writes() {
        assert!(!reads("postgres", "delete from t"));
        assert!(!reads("postgres", "  DROP TABLE t"));
        assert!(!reads("postgres", "insert into t values (1)"));
        assert!(!reads("postgres", "update t set a = 1"));
        assert!(!reads("neo4j", "CREATE (n:Node)"));
        assert!(!reads(
            "snowflake",
            "merge into t using s on t.a = s.a when matched then delete"
        ));
        assert_eq!(access("mqtt", "publish a b"), Access::Write);
        assert_eq!(access("s3", "rmb b"), Access::Write);
    }

    #[test]
    fn stops_writes_hidden_behind_a_read() {
        let hidden = [
            ("neo4j", "MATCH (n) DETACH DELETE n"),
            ("falkordb", "match (n) set n.x = 1"),
            ("neo4j", "CALL apoc.periodic.iterate('a', 'b', {})"),
            ("duckdb", "with x as (select 1) delete from t"),
            ("turso", "with x as (select 1) delete from t"),
            ("d1", "with x as (select 1) delete from t"),
            ("duckdb", "explain analyze delete from t"),
            ("postgres", "explain (analyze, buffers) delete from t"),
            (
                "postgres",
                "with d as (delete from t returning *) select * from d",
            ),
            ("postgres", "select 1; delete from t"),
            ("postgres", "select * from t for update"),
            ("postgres", "select * into copy from t"),
            (
                "postgres",
                "select pg_catalog.set_config('default_transaction_read_only', 'off', false)",
            ),
            ("postgres", "set default_transaction_read_only = off"),
            (
                "postgres",
                "set session characteristics as transaction read write",
            ),
            ("postgres", "begin read write"),
            ("postgres", "reset all"),
            ("postgres", "discard all"),
            ("postgres", "copy t to '/tmp/x'"),
            ("mysql", "set @@transaction_read_only = 0"),
            ("mysql", "set session tx_read_only = 0"),
            ("clickhouse", "set readonly = 0"),
            ("sqlite", "pragma query_only(0)"),
            ("sqlite", "pragma query_only = 0"),
            ("sqlite", "pragma incremental_vacuum"),
            ("sqlite", "vacuum into '/tmp/copy.db'"),
            ("sqlite", "attach database '/tmp/x.db' as x"),
            ("duckdb", "copy t to 'out.csv'"),
            ("duckdb", "install httpfs"),
            (
                "influxdb2",
                "from(bucket:\"a\") |> range(start:0) |> to(bucket:\"b\")",
            ),
            (
                "influxdb2",
                "import \"experimental\"\nfrom(bucket:\"a\") |> experimental.to(bucket:\"b\")",
            ),
            ("influxdb2", "f = to\nfrom(bucket:\"a\") |> f(bucket:\"b\")"),
            (
                "influxdb2",
                "from(bucket:\"a\") |> filter(fn: (r) => r.u =~ /a\\/\\//) |> to(bucket:\"b\")",
            ),
        ];

        for (kind, sql) in hidden {
            assert!(!reads(kind, sql), "{kind}: {sql}");
        }
    }

    #[test]
    fn an_unknown_import_or_a_parse_failure_is_not_a_read() {
        assert_eq!(
            access("influxdb2", "import \"http\"\nhttp.post(url: \"x\")"),
            Access::Unknown
        );
        assert_eq!(access("turso", "selec from where"), Access::Unknown);
        assert_eq!(
            access("neo4j", "MATCH (n) WHERE n.a = 'open"),
            Access::Unknown
        );
    }

    #[test]
    fn names_that_only_look_like_clauses_are_reads() {
        assert!(reads(
            "neo4j",
            "MATCH (n:Set {create: 1}) RETURN n.delete, $merge"
        ));
        assert!(reads(
            "neo4j",
            "MATCH (n) WHERE n.name = 'DELETE' RETURN n // drop"
        ));
        assert!(reads("neo4j", "CALL db.labels() YIELD label RETURN label"));
        assert!(reads("neo4j", "MATCH (n:`CREATE`) RETURN n"));
        assert!(reads(
            "influxdb2",
            "from(bucket:\"a\") |> range(start:0) |> filter(fn: (r) => r.to == 1)"
        ));
        assert!(reads(
            "influxdb2",
            "// latest readings to check\nfrom(bucket:\"b\") |> range(start:-1h)"
        ));
        assert!(reads(
            "influxdb2",
            "from(bucket:\"b\") |> range(start:-1h) |> map(fn: (r) => ({r with to: 1}))"
        ));
    }

    #[test]
    fn snowflake_system_functions_are_writes_unless_known_to_read() {
        assert_eq!(
            access("snowflake", "select system$cancel_all_queries(1)"),
            Access::Write
        );
        assert_eq!(
            access("snowflake", "select SYSTEM$ABORT_SESSION(1)"),
            Access::Write
        );
        assert!(reads("snowflake", "select system$typeof(1)"));
    }

    #[test]
    fn a_script_the_parser_rejects_is_trusted_only_as_one_plain_read() {
        let refused = [
            (
                "postgres",
                "begin not deferrable, read write; delete from t; commit",
            ),
            (
                "mysql",
                "set session transaction_read_only := 0; delete from t",
            ),
            ("mysql", "set persist read_only = 0"),
            ("mysql", "set global read_only := 0"),
            ("duckdb", "export database 'dir'"),
            ("duckdb", "import database 'dir'"),
            ("mysql", "select * from t into outfile '/tmp/x'"),
            ("mysql", "select * from t into dumpfile '/tmp/x'"),
            ("postgres", "select lo_export(1, '/tmp/x')"),
            ("postgres", "explain (analyze) delete from t"),
        ];

        for (kind, sql) in refused {
            assert_ne!(access(kind, sql), Access::Read, "{kind}: {sql}");
            assert!(!plain_read(kind, sql), "{kind}: {sql}");
        }

        assert!(!plain_read("postgres", "select 1; select 2"));
        assert!(plain_read("postgres", "select 1 -- ; delete from t\n;"));
        assert!(plain_read("mysql", "select ';delete' from t"));
    }

    #[test]
    fn transaction_control_is_found_by_its_tokens() {
        let cases = [
            (
                "postgres",
                "begin not deferrable, read write; delete from t; commit",
                Ending::Mixed,
            ),
            ("postgres", "commit", Ending::Commit),
            ("sqlite", "end transaction;", Ending::Commit),
            ("postgres", "rollback work", Ending::Rollback),
            ("postgres", "select 1; commit", Ending::Mixed),
            ("postgres", "savepoint a", Ending::Savepoint),
            ("postgres", "rollback to savepoint a", Ending::Savepoint),
            ("mysql", "set autocommit = 0", Ending::Mixed),
            ("mysql", "xa start 'x'", Ending::Mixed),
            ("postgres", "select 'begin; commit'", Ending::None),
            ("postgres", "select * from t", Ending::None),
            (
                "sqlite",
                "create trigger x after insert on t begin insert into l values (1); end",
                Ending::None,
            ),
            (
                "mysql",
                "create procedure p() begin declare x int; if x then select 1; end if; select case when x then 1 end; end",
                Ending::None,
            ),
            (
                "postgres",
                "select case when a then case when b then 1 end end from t; commit",
                Ending::Mixed,
            ),
        ];

        for (kind, sql, expected) in cases {
            assert_eq!(ending(kind, sql), expected, "{kind}: {sql}");
        }
    }

    #[test]
    fn mysql_statements_that_commit_by_themselves_are_spotted() {
        let committing = [
            "create table t (a int)",
            "alter table t add b int",
            "drop table t",
            "truncate t",
            "rename table a to b",
            "lock tables t write",
            "grant select on *.* to u",
            "analyze table t",
            "set autocommit = 1",
            "insert into t values (1); create index i on t (a)",
        ];

        for sql in committing {
            assert!(!keeps_transaction("mysql", sql), "{sql}");
        }

        let keeping = [
            "insert into t values (1)",
            "update t set a = 1",
            "select * from t",
            "create temporary table x (a int)",
            "set @a = 1",
            "savepoint a",
        ];

        for sql in keeping {
            assert!(keeps_transaction("mysql", sql), "{sql}");
        }

        assert!(calls_routine("mysql", "insert into t values (1); CALL p()"));
        assert!(!calls_routine(
            "mysql",
            "create procedure p() begin call q(); end"
        ));
    }

    fn scratch(name: &str) -> String {
        let path = std::env::temp_dir().join(format!("gpql-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&path);

        path.to_string_lossy().into_owned()
    }

    async fn opened(kind: &str, path: &str) -> Session {
        let config = SessionConfig {
            kind: kind.into(),
            path: path.into(),
            create: true,
            ..Default::default()
        };

        open(&config).await.unwrap()
    }

    #[tokio::test]
    async fn a_read_only_sqlite_file_refuses_writes_whatever_is_typed() {
        let path = scratch("ro.db");
        let session = opened("sqlite", &path).await;

        read(&session, "create table t (a int)").await.unwrap();
        set_read_only(&session, true).await.unwrap();

        assert!(session.guarded());

        read(&session, "pragma query_only = 0").await.unwrap();
        assert!(read(&session, "insert into t values (1)").await.is_err());

        let mode = read(&session, "pragma journal_mode = wal").await;
        let mode = mode
            .ok()
            .and_then(|result| result.rows.first()?.first()?.clone());
        assert_ne!(mode.as_deref(), Some("wal"));

        set_read_only(&session, false).await.unwrap();
        read(&session, "insert into t values (1)").await.unwrap();

        read(&session, "begin").await.unwrap();
        assert!(set_read_only(&session, true).await.is_err());
        assert!(!session.guarded());
        read(&session, "rollback").await.unwrap();

        drop(session);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn duckdb_keeps_a_transaction_through_a_binder_error_but_not_a_constraint_error() {
        let path = scratch("abort.duckdb");
        let session = opened("duckdb", &path).await;

        read(&session, "create table t (a int primary key)")
            .await
            .unwrap();
        set_manual(&session, true).await.unwrap();

        query(&session, "insert into t values (1)").await.unwrap();
        assert!(query(&session, "select nope from t").await.is_err());
        assert!(finish(&session, "commit").await.is_ok());

        query(&session, "insert into t values (2)").await.unwrap();
        assert!(query(&session, "insert into t values (1)").await.is_err());
        assert!(finish(&session, "commit").await.is_err());

        set_manual(&session, false).await.unwrap();

        let kept = read(&session, "select count(*) from t").await.unwrap();
        assert_eq!(kept.rows[0][0].as_deref(), Some("1"));

        drop(session);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn typed_transaction_control_needs_manual_commit() {
        let path = scratch("typed.db");
        let session = opened("sqlite", &path).await;

        let refused = query(&session, "begin").await.err().unwrap_or_default();
        assert!(refused.contains("turn on manual commit"));
        assert!(query(&session, "savepoint a").await.is_err());

        drop(session);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn gpql_reads_its_own_browsing_queries() {
        let generated = [
            ("postgres", "select * from \"t\" where \"a\" like '%x!%%' escape '!' order by \"id\" asc limit 10 offset 0"),
            ("postgres", "select * from \"t\" where \"a\" = E'C:\\\\new' limit 10 offset 0"),
            ("mysql", "select `a` from `t` where `a` = 'it''s \\\\' order by `id` limit 10 offset 0"),
            ("sqlite", "select * from \"t\" limit 10 offset 0"),
            ("d1", "select * from `t` where `a` is not null limit 10 offset 0"),
            ("turso", "select name from sqlite_master where type = 'table' and name not like 'sqlite_%' order by name"),
            ("turso", "pragma table_info('t')"),
            ("duckdb", "select table_name, estimated_size from duckdb_tables() order by table_name"),
            ("clickhouse", "select * from \"t\" where \"a\" like '%x\\\\%%' order by \"id\" limit 10 offset 0"),
            ("clickhouse", "show tables"),
            ("snowflake", "show tables"),
            ("snowflake", "select current_version()"),
            ("influxdb", "select table_name from information_schema.tables where table_schema = 'iox' order by table_name"),
            ("influxdb", "explain analyze select 1"),
            ("neo4j", "match (n:`Person`) where n.`name` = 'x' return n skip 0 limit 10"),
            ("falkordb", "call db.labels()"),
            ("influxdb2", "import \"strings\"\n\nfrom(bucket: \"b\")\n  |> range(start: 0)\n  |> filter(fn: (r) => strings.containsStr(v: r[\"host\"], substr: \"edge\"))"),
            ("sqlite", "select 'sqlite ' || sqlite_version()"),
            ("neo4j", "return 1"),
            ("postgres", "select datname from pg_database where datistemplate = false order by 1"),
            ("mysql", "show databases"),
            ("mysql", "show create table `t`"),
            ("s3", "ls b"),
        ];

        for (kind, sql) in generated {
            assert!(reads(kind, sql), "{kind}: {sql}");
        }
    }

    #[test]
    fn analyze_takes_one_read_only() {
        assert!(single_read("postgres", "select * from t"));
        assert!(!single_read("postgres", "select 1; delete from t"));
        assert!(!single_read(
            "postgres",
            "with d as (delete from t returning *) select * from d"
        ));
    }
}

#[cfg(test)]
#[path = "db_tests.rs"]
mod db_tests;
