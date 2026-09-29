mod editor;
mod engines;
mod net;
mod store;

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::editor::highlight;
use crate::editor::lsp;
use crate::engines::backends;
use crate::engines::db;
use crate::engines::ddl;
use crate::engines::discovery;
use crate::engines::export;
use crate::engines::objects;
use crate::engines::plan;
use crate::net::login;
use crate::net::tailnet;
use crate::net::tunnel::{self, Tunnels};
use crate::store::local;
use crate::store::vault;

use crate::engines::introspect;
use crate::engines::slicing;
use crate::engines::writing;
use db::{QueryResult, SessionConfig, SessionHandle, Sessions, TableInfo, TableSchema};
use discovery::Discovery;
use highlight::{Highlighter, Token};
use local::Local;
use lsp::{Completion, Servers};
use serde_json::Value as Json;
use tauri::{AppHandle, Emitter, Manager, State};
use vault::{Credential, Provider, SavedLogin};

async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
}

type Stops = HashMap<u64, tokio::sync::oneshot::Sender<()>>;

#[derive(Default)]
struct Busy {
    next: AtomicU64,
    running: Mutex<HashMap<String, Stops>>,
}

struct Ticket<'a> {
    busy: &'a Busy,
    session: String,
    number: u64,
}

impl Drop for Ticket<'_> {
    fn drop(&mut self) {
        let mut running = self.busy.held();

        if let Some(stops) = running.get_mut(&self.session) {
            stops.remove(&self.number);

            if stops.is_empty() {
                running.remove(&self.session);
            }
        }
    }
}

impl Busy {
    fn held(&self) -> MutexGuard<'_, HashMap<String, Stops>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    async fn watch<T>(
        &self,
        session: &str,
        work: impl Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        let number = self.next.fetch_add(1, Ordering::Relaxed);
        let (stop, stopped) = tokio::sync::oneshot::channel();

        self.held()
            .entry(session.to_string())
            .or_default()
            .insert(number, stop);

        let _ticket = Ticket {
            busy: self,
            session: session.to_string(),
            number,
        };

        tokio::select! {
            outcome = work => outcome,
            Ok(()) = stopped => Err("the query was stopped".to_string()),
        }
    }

    fn running(&self, session: &str) -> bool {
        self.held().contains_key(session)
    }

    fn stop(&self, session: &str) -> bool {
        let stops = self.held().remove(session).unwrap_or_default();
        let stopped = !stops.is_empty();

        for stop in stops.into_values() {
            let _ = stop.send(());
        }

        stopped
    }
}

#[tauri::command]
async fn check(config: SessionConfig, tunnels: State<'_, Tunnels>) -> Result<String, String> {
    let mut asked = config.clone();

    asked.create = false;

    // the probe has to take the same route the connection will, or it reports
    // on a server the driver is never going to reach
    let (reached, _hop) = through(&asked, &tunnels).await?;
    let session = db::open(&reached).await?;

    let probe = match (config.kind.as_str(), backends::dialect_of(&config.kind)) {
        ("sqlite", _) => "select 'sqlite ' || sqlite_version()",
        (_, "cypher") => "return 1",
        (_, "flux") => {
            let listed = introspect::tables(&session).await?;
            let what = if config.database.is_empty() {
                "buckets"
            } else {
                "measurements"
            };

            return Ok(format!("{} {what}", listed.len()));
        }
        ("mqtt", _) => {
            let listed = introspect::tables(&session).await?;

            return Ok(format!("{} topics", listed.len()));
        }
        ("s3", _) => "ls",
        ("clickhouse", _) => "select version()",
        ("snowflake", _) => "select current_version()",
        ("influxdb", _) => "select 1",
        ("turso" | "d1", _) => "select sqlite_version()",
        _ => "select version()",
    };

    let result = db::query(&session, probe).await?;

    Ok(result
        .rows
        .first()
        .and_then(|row| row.first().cloned().flatten())
        .unwrap_or_else(|| "reachable".into()))
}

// the driver is pointed at a loopback port and never learns there is a jump
// host in front of the server
async fn through(
    config: &SessionConfig,
    tunnels: &Tunnels,
) -> Result<(SessionConfig, Option<tunnel::Tunnel>), String> {
    if !config.tunnel.wanted() {
        return Ok((config.clone(), None));
    }

    let mut reached = db::flavoured(config);

    // a url backend carries its address inside the url, so the hop is dialled
    // from there and the url is rewritten to the loopback port
    if !reached.url.trim().is_empty() {
        let mut address = url::Url::parse(reached.url.trim())
            .map_err(|error| format!("that URL cannot be read: {error}"))?;
        let host = address
            .host_str()
            .ok_or_else(|| "that URL names no host to reach".to_string())?
            .to_string();
        let port = address
            .port_or_known_default()
            .ok_or_else(|| "that URL names no port to reach".to_string())?;

        let hop = tunnel::open(tunnels, &config.tunnel, &host, port).await?;

        address
            .set_host(Some("127.0.0.1"))
            .map_err(|error| error.to_string())?;
        address
            .set_port(Some(hop.local_port))
            .map_err(|_| "that URL will not take a port".to_string())?;

        reached.url = address.to_string();

        return Ok((reached, Some(hop)));
    }

    let target = if reached.host.trim().is_empty() {
        "127.0.0.1".to_string()
    } else {
        reached.host.trim().to_string()
    };
    let port = reached.port.trim().parse().unwrap_or_else(|_| {
        crate::backends::find(&reached.kind)
            .and_then(|backend| backend.port.parse().ok())
            .unwrap_or(5432)
    });
    let hop = tunnel::open(tunnels, &reached.tunnel, &target, port).await?;

    // tls still checks the certificate against the real host name, so only the dialled address changes
    match backends::transport_of(&reached.kind) {
        backends::Transport::Postgres | backends::Transport::MySql => {
            reached.host = target;
            reached.dial = "127.0.0.1".into();
        }
        _ => reached.host = "127.0.0.1".into(),
    }

    reached.port = hop.local_port.to_string();

    Ok((reached, Some(hop)))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Waypoint {
    url: String,
    kind: String,
}

#[tauri::command]
async fn probe_recents(items: Vec<Waypoint>) -> Vec<String> {
    let logins = off_thread(vault::list).await.unwrap_or_default();

    let checks = items.into_iter().map(|item| {
        if item.kind == "erd" {
            return Some(Look::File(item.url));
        }

        let login = logins
            .iter()
            .find(|saved| saved.url == item.url)
            .or_else(|| {
                logins
                    .iter()
                    .find(|saved| tail(&saved.url) == tail(&item.url))
            })?;

        if !login.path.is_empty() {
            return Some(Look::File(login.path.clone()));
        }

        // only the jump host can be reached from here; the server behind it is not probed
        if login.tunnel.wanted() {
            return Some(Look::Port(
                login.tunnel.host.trim().to_string(),
                login.tunnel.port.trim().parse().unwrap_or(22),
            ));
        }

        if login.endpoint.is_empty() {
            let fallback = crate::backends::find(&login.kind)
                .map(|backend| backend.port)
                .filter(|port| !port.is_empty())
                .unwrap_or("5432");

            return address_of(&format!(
                "{}:{}",
                login.host,
                if login.port.is_empty() {
                    fallback
                } else {
                    &login.port
                }
            ))
            .map(|(host, port)| Look::Port(host, port));
        }

        address_of(&login.endpoint).map(|(host, port)| Look::Port(host, port))
    });

    let answers = checks.map(|check| async move {
        match check {
            None => String::new(),
            Some(Look::File(path)) => off_thread(move || Ok(missing_file(&path)))
                .await
                .unwrap_or_default(),
            Some(Look::Port(host, _)) if host.is_empty() => String::new(),
            Some(Look::Port(host, port)) => {
                if discovery::reachable(&host, port, 400).await {
                    String::new()
                } else {
                    "down".to_string()
                }
            }
        }
    });

    futures_util::future::join_all(answers).await
}

enum Look {
    File(String),
    Port(String, u16),
}

fn tail(url: &str) -> &str {
    match url.find("://") {
        Some(at) => &url[at + 3..],
        None => url,
    }
}

fn missing_file(path: &str) -> String {
    if std::path::Path::new(path).exists() {
        return String::new();
    }

    "gone".to_string()
}

fn address_of(endpoint: &str) -> Option<(String, u16)> {
    let secure = endpoint.starts_with("https://");
    let bare = endpoint
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = bare.split('/').next()?;

    if let Some((name, port)) = host.rsplit_once(':') {
        if let Ok(port) = port.parse::<u16>() {
            return Some((name.to_string(), port));
        }
    }

    Some((host.to_string(), if secure { 443 } else { 80 }))
}

#[tauri::command]
async fn databases(
    config: SessionConfig,
    tunnels: State<'_, Tunnels>,
) -> Result<Vec<String>, String> {
    use crate::backends::Transport;

    if config.kind == "supabase_api" {
        return supabase_projects(&config.token).await;
    }

    let mut probing = config.clone();

    probing.read_only = true;
    probing.create = false;

    if config.kind == "influxdb2" {
        let (reached, _hop) = through(&probing, &tunnels).await?;
        let session = db::open(&reached).await?;

        return match &session.engine {
            db::Engine::Driver(driver) => driver.databases().await,
            _ => Ok(Vec::new()),
        };
    }

    let listing = match crate::backends::transport_of(&config.kind) {
        Transport::Postgres => {
            "select datname from pg_database where datistemplate = false order by 1"
        }
        Transport::MySql => "show databases",
        _ => return Ok(Vec::new()),
    };

    if probing.database.is_empty() {
        probing.database = match crate::backends::transport_of(&config.kind) {
            Transport::MySql => "information_schema".into(),
            _ => "postgres".into(),
        };
    }

    let (reached, _hop) = through(&probing, &tunnels).await?;
    let session = db::open(&reached).await?;
    let result = db::query(&session, listing).await?;

    Ok(result
        .rows
        .into_iter()
        .filter_map(|row| row.into_iter().next().flatten())
        .collect())
}

async fn supabase_projects(token: &str) -> Result<Vec<String>, String> {
    if token.is_empty() {
        return Ok(Vec::new());
    }

    let response = reqwest::Client::new()
        .get("https://api.supabase.com/v1/projects")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    Ok(net::answer(response)
        .await?
        .as_array()
        .map(|projects| {
            projects
                .iter()
                .filter_map(|project| project.get("id").and_then(Json::as_str))
                .map(|id| id.to_string())
                .collect()
        })
        .unwrap_or_default())
}

#[tauri::command]
async fn connect(
    config: SessionConfig,
    app: AppHandle,
    sessions: State<'_, Sessions>,
    tunnels: State<'_, Tunnels>,
) -> Result<SessionHandle, String> {
    let (reached, hop) = through(&config, &tunnels).await?;
    let session = db::open(&reached).await?;
    let saving = config.clone();

    off_thread(move || vault::remember(&saving)).await?;

    let handle = sessions.insert(session);

    if let Some(hop) = hop {
        tunnels.keep(&handle.id, hop);
    }

    if let Ok(open) = sessions.get(&handle.id) {
        let app = app.clone();
        let target = handle.id.clone();

        open.on_catalog_change(move |topic| {
            let _ = app.emit(
                "catalog-changed",
                serde_json::json!({ "session": target, "topic": topic }),
            );
        });
    }

    Ok(handle)
}

#[tauri::command]
async fn disconnect(
    id: String,
    sessions: State<'_, Sessions>,
    tunnels: State<'_, Tunnels>,
) -> Result<(), String> {
    // an uncommitted transaction would otherwise hold locks until the server
    // notices the socket is gone
    if let Ok(session) = sessions.get(&id) {
        let _ = writing::finish(&session, "rollback").await;
    }

    sessions.remove(&id);
    tunnels.drop_for(&id);

    Ok(())
}

#[tauri::command]
async fn export_table(
    id: String,
    table: String,
    slice: slicing::Slice,
    format: export::Format,
    path: String,
    sessions: State<'_, Sessions>,
) -> Result<u64, String> {
    let session = sessions.get(&id)?;

    return export::export_table(&session, &table, &slice, format, &path).await;
}

#[tauri::command]
async fn export_result(
    id: String,
    result: db::QueryResult,
    table: String,
    format: export::Format,
    path: String,
    sessions: State<'_, Sessions>,
) -> Result<u64, String> {
    let session = sessions.get(&id)?;

    return export::export_result(&session, &result, &table, format, &path).await;
}

#[tauri::command]
async fn objects(
    id: String,
    sessions: State<'_, Sessions>,
) -> Result<Vec<objects::DbObject>, String> {
    let session = sessions.get(&id)?;

    return objects::objects(&session).await;
}

#[tauri::command]
async fn object_ddl(
    id: String,
    name: String,
    kind: Option<String>,
    detail: Option<String>,
    sessions: State<'_, Sessions>,
) -> Result<String, String> {
    let session = sessions.get(&id)?;

    return ddl::object_ddl(&session, &name, kind.as_deref(), detail.as_deref()).await;
}

#[tauri::command]
async fn mqtt_publish(
    id: String,
    topic: String,
    payload: String,
    qos: u8,
    retain: bool,
    sessions: State<'_, Sessions>,
) -> Result<(), String> {
    let session = sessions.get(&id)?;

    if session.read_only.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("this session is read only".into());
    }

    return session.mqtt()?.publish(&topic, &payload, qos, retain).await;
}

#[tauri::command]
async fn mqtt_clear(
    id: String,
    topic: String,
    app: AppHandle,
    sessions: State<'_, Sessions>,
) -> Result<(), String> {
    let session = sessions.get(&id)?;

    session.mqtt()?.clear(&topic);

    let _ = app.emit(
        "catalog-changed",
        serde_json::json!({ "session": id, "topic": topic }),
    );

    Ok(())
}

#[tauri::command]
async fn mqtt_subscribe(
    id: String,
    filter: String,
    qos: u8,
    sessions: State<'_, Sessions>,
) -> Result<(), String> {
    let session = sessions.get(&id)?;

    return session.mqtt()?.subscribe(&filter, qos).await;
}

#[tauri::command]
async fn mqtt_unsubscribe(
    id: String,
    filter: String,
    sessions: State<'_, Sessions>,
) -> Result<(), String> {
    let session = sessions.get(&id)?;

    return session.mqtt()?.unsubscribe(&filter).await;
}

#[tauri::command]
async fn s3_presign(
    id: String,
    bucket: String,
    key: String,
    sessions: State<'_, Sessions>,
) -> Result<String, String> {
    let session = sessions.get(&id)?;

    return session.s3()?.presign(&bucket, &key).await;
}

#[tauri::command]
async fn s3_download(
    id: String,
    bucket: String,
    key: String,
    path: String,
    sessions: State<'_, Sessions>,
) -> Result<u64, String> {
    let session = sessions.get(&id)?;

    return session.s3()?.download(&bucket, &key, &path).await;
}

#[tauri::command]
async fn s3_upload(
    id: String,
    bucket: String,
    key: String,
    path: String,
    app: AppHandle,
    sessions: State<'_, Sessions>,
) -> Result<(), String> {
    let session = sessions.get(&id)?;

    if session.read_only.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("this session is read only".into());
    }

    session.s3()?.upload(&bucket, &key, &path).await?;

    let _ = app.emit(
        "catalog-changed",
        serde_json::json!({ "session": id, "topic": bucket }),
    );

    Ok(())
}

#[tauri::command]
async fn s3_delete(
    id: String,
    bucket: String,
    key: String,
    app: AppHandle,
    sessions: State<'_, Sessions>,
) -> Result<(), String> {
    let session = sessions.get(&id)?;

    if session.read_only.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("this session is read only".into());
    }

    session.s3()?.delete(&bucket, &key).await?;

    let _ = app.emit(
        "catalog-changed",
        serde_json::json!({ "session": id, "topic": bucket }),
    );

    Ok(())
}

#[tauri::command]
async fn s3_refresh(
    id: String,
    bucket: String,
    sessions: State<'_, Sessions>,
) -> Result<(), String> {
    let session = sessions.get(&id)?;

    return session.s3()?.refresh(&bucket).await;
}

#[tauri::command]
async fn mqtt_subscriptions(
    id: String,
    sessions: State<'_, Sessions>,
) -> Result<Vec<crate::engines::mqtt::Subscription>, String> {
    let session = sessions.get(&id)?;

    Ok(session.mqtt()?.subscriptions())
}

#[tauri::command]
async fn explain_query(
    id: String,
    sql: String,
    analyze: bool,
    sessions: State<'_, Sessions>,
    busy: State<'_, Busy>,
) -> Result<plan::Plan, String> {
    let session = sessions.get(&id)?;

    busy.watch(&id, plan::explain(&session, &sql, analyze))
        .await
}

#[tauri::command]
fn reset_sessions(sessions: State<'_, Sessions>, tunnels: State<'_, Tunnels>) {
    sessions.clear();
    tunnels.clear();
}

#[tauri::command]
async fn tables(id: String, sessions: State<'_, Sessions>) -> Result<Vec<TableInfo>, String> {
    let session = sessions.get(&id)?;

    return introspect::tables(&session).await;
}

#[tauri::command]
async fn schemas(id: String, sessions: State<'_, Sessions>) -> Result<Vec<String>, String> {
    let session = sessions.get(&id)?;

    return introspect::schemas(&session).await;
}

#[tauri::command]
async fn use_schema(id: String, name: String, sessions: State<'_, Sessions>) -> Result<(), String> {
    let session = sessions.get(&id)?;

    return introspect::use_schema(&session, &name).await;
}

#[tauri::command]
async fn check_sql(
    sql: String,
    dialect: String,
    app: AppHandle,
) -> Result<Option<highlight::Fault>, String> {
    off_thread(move || Ok(app.state::<Highlighter>().fault(&dialect, &sql))).await
}

#[tauri::command]
async fn set_manual(id: String, on: bool, sessions: State<'_, Sessions>) -> Result<(), String> {
    let session = sessions.get(&id)?;

    return writing::set_manual(&session, on).await;
}

#[tauri::command]
async fn end_transaction(
    id: String,
    commit: bool,
    sessions: State<'_, Sessions>,
) -> Result<bool, String> {
    let session = sessions.get(&id)?;

    return writing::finish(&session, if commit { "commit" } else { "rollback" }).await;
}

#[tauri::command]
async fn pending_edits(
    id: String,
    table: String,
    edits: Vec<writing::Edit>,
    sessions: State<'_, Sessions>,
) -> Result<Vec<String>, String> {
    let session = sessions.get(&id)?;

    writing::edit_statements(&session, &table, &edits).await
}

#[tauri::command]
async fn set_read_only(id: String, on: bool, sessions: State<'_, Sessions>) -> Result<(), String> {
    let session = sessions.get(&id)?;

    return writing::set_read_only(&session, on).await;
}

#[tauri::command]
async fn built_query(
    id: String,
    table: String,
    slice: slicing::Slice,
    shape: slicing::Shape,
    sessions: State<'_, Sessions>,
) -> Result<String, String> {
    let session = sessions.get(&id)?;

    return Ok(slicing::shaped_query(&session, &table, &slice, &shape).await);
}

#[tauri::command]
async fn table_rows(
    id: String,
    table: String,
    slice: slicing::Slice,
    sessions: State<'_, Sessions>,
    busy: State<'_, Busy>,
) -> Result<QueryResult, String> {
    let session = sessions.get(&id)?;

    busy.watch(&id, slicing::table_rows(&session, &table, &slice))
        .await
}

#[tauri::command]
async fn run_query(
    id: String,
    sql: String,
    sessions: State<'_, Sessions>,
    busy: State<'_, Busy>,
) -> Result<QueryResult, String> {
    let session = sessions.get(&id)?;

    busy.watch(&id, db::query(&session, &sql)).await
}

#[tauri::command]
async fn cancel_query(
    session_id: String,
    sessions: State<'_, Sessions>,
    busy: State<'_, Busy>,
) -> Result<bool, String> {
    if !busy.running(&session_id) {
        return Ok(false);
    }

    let session = sessions.get(&session_id)?;

    // a server-side cancel lets the statement fail with the server's own words
    if session.cancel().await.is_ok() {
        return Ok(true);
    }

    Ok(busy.stop(&session_id))
}

#[tauri::command]
async fn apply_edits(
    id: String,
    table: String,
    edits: Vec<writing::Edit>,
    sessions: State<'_, Sessions>,
) -> Result<u64, String> {
    let session = sessions.get(&id)?;

    return writing::apply(&session, &table, &edits).await;
}

#[tauri::command]
async fn schema(id: String, sessions: State<'_, Sessions>) -> Result<Vec<TableSchema>, String> {
    let session = sessions.get(&id)?;

    return introspect::schema(&session).await;
}

#[tauri::command]
async fn highlight_sql(sql: String, dialect: String, app: AppHandle) -> Result<Vec<Token>, String> {
    off_thread(move || Ok(app.state::<Highlighter>().tokens(&dialect, &sql))).await
}

#[tauri::command]
async fn lsp_start(
    dialect: String,
    program: String,
    args: Vec<String>,
    servers: State<'_, Servers>,
) -> Result<(), String> {
    return servers.start(&dialect, &program, &args).await;
}

#[tauri::command]
async fn lsp_stop(dialect: String, servers: State<'_, Servers>) -> Result<(), String> {
    servers.stop(&dialect).await;

    Ok(())
}

#[tauri::command]
async fn lsp_running(servers: State<'_, Servers>) -> Result<Vec<String>, String> {
    return Ok(servers.running().await);
}

#[tauri::command]
async fn lsp_complete(
    dialect: String,
    text: String,
    line: u32,
    character: u32,
    servers: State<'_, Servers>,
) -> Result<Vec<Completion>, String> {
    servers.sync(&dialect, &text).await.ok();

    return servers.complete(&dialect, line, character).await;
}

const DOCUMENT_EXTENSION: &str = "gpqlerd";

fn document(path: &str) -> Result<&std::path::Path, String> {
    let path = std::path::Path::new(path);
    let ours = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(DOCUMENT_EXTENSION));

    if !ours {
        return Err(format!(
            "only .{DOCUMENT_EXTENSION} documents can be opened or saved here"
        ));
    }

    Ok(path)
}

#[tauri::command]
fn read_document(path: String) -> Result<String, String> {
    std::fs::read_to_string(document(&path)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_document(path: String, text: String) -> Result<(), String> {
    std::fs::write(document(&path)?, text).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_acrylic(window: tauri::Window, on: bool, dark: bool) -> Result<(), String> {
    use tauri::utils::config::WindowEffectsConfig;
    use tauri::window::Effect;

    let tinted = if dark {
        Effect::MicaDark
    } else {
        Effect::MicaLight
    };

    let effects = on.then(|| WindowEffectsConfig {
        effects: vec![Effect::Acrylic, tinted],
        state: None,
        radius: None,
        color: None,
    });

    let theme = if dark {
        tauri::Theme::Dark
    } else {
        tauri::Theme::Light
    };

    window.set_theme(Some(theme)).map_err(|e| e.to_string())?;

    window.set_effects(effects).map_err(|e| e.to_string())
}

#[tauri::command]
async fn look_on_this_machine() -> Vec<u16> {
    discovery::local_postgres_ports().await
}

fn keyring() -> discovery::Keyring {
    let saved = vault::list()
        .unwrap_or_default()
        .into_iter()
        .filter(|login| !login.tunnel.wanted() && !login.user.is_empty())
        .filter_map(|login| {
            let (host, port) = if login.endpoint.is_empty() {
                let port = if login.port.is_empty() {
                    backends::find(&login.kind)?.port
                } else {
                    login.port.as_str()
                };

                (login.host.clone(), port.parse().ok()?)
            } else {
                address_of(&login.endpoint)?
            };

            Some(discovery::Saved {
                host,
                port,
                key: discovery::Key {
                    user: login.user,
                    password: login.password,
                    tls: login.tls,
                },
            })
        })
        .collect();

    let presets = vault::credentials()
        .unwrap_or_default()
        .into_iter()
        .map(|preset| discovery::Key {
            user: preset.user,
            password: preset.password,
            tls: String::new(),
        })
        .collect();

    discovery::Keyring { saved, presets }
}

#[tauri::command]
async fn scan_local() -> Vec<Discovery> {
    let keys = off_thread(|| Ok(keyring())).await.unwrap_or_default();

    discovery::scan(&keys).await
}

#[tauri::command]
async fn scan_tailnet() -> Result<Vec<Discovery>, String> {
    let hosts: Vec<String> = tailnet::peers()
        .await?
        .into_iter()
        .filter(|peer| peer.online)
        .map(|peer| peer.host)
        .collect();
    let keys = off_thread(|| Ok(keyring())).await?;

    Ok(discovery::scan_hosts(&hosts, &[5432, 5433], &keys).await)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedErd {
    id: String,
    link: String,
    open: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ErdRoom {
    id: String,
    name: String,
    open: bool,
    created_at: Json,
}

impl ErdRoom {
    fn read(room: &Json) -> Option<Self> {
        let open = room.get("open").unwrap_or(&Json::Null);

        Some(ErdRoom {
            id: room.get("id")?.as_str()?.to_string(),
            name: room
                .get("name")
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_string(),
            open: open
                .as_bool()
                .or_else(|| open.as_i64().map(|flag| flag != 0))
                .unwrap_or(false),
            created_at: room.get("createdAt").cloned().unwrap_or(Json::Null),
        })
    }
}

async fn signed_in() -> Result<String, String> {
    off_thread(|| Ok(vault::account_token()))
        .await?
        .ok_or_else(|| "sign in first".to_string())
}

fn erd_endpoint(site: &str) -> String {
    format!("{}/api/erd", site.trim_end_matches('/'))
}

async fn sent(request: reqwest::RequestBuilder) -> Result<Json, String> {
    let response = request.send().await.map_err(|error| error.to_string())?;

    net::answer(response).await
}

#[tauri::command]
async fn publish_schema(
    site: String,
    name: String,
    sessions: State<'_, Sessions>,
    session_id: String,
    id: Option<String>,
) -> Result<SharedErd, String> {
    let token = signed_in().await?;
    let session = sessions.get(&session_id)?;
    let tables = introspect::schema(&session).await?;
    let client = reqwest::Client::new();
    let endpoint = erd_endpoint(&site);
    let wanted = id.filter(|id| !id.trim().is_empty());

    let mut updated = None;

    if let Some(id) = &wanted {
        let response = client
            .put(&endpoint)
            .bearer_auth(&token)
            .json(&serde_json::json!({ "id": id, "name": name, "tables": tables }))
            .send()
            .await
            .map_err(|error| error.to_string())?;

        // a room deleted on the web answers 404 and a site without PUT answers 405; both start afresh
        let gone = matches!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::METHOD_NOT_ALLOWED
        );

        if !gone {
            updated = Some(net::answer(response).await?);
        }
    }

    let (answer, requested) = match updated {
        Some(answer) => (answer, wanted),
        None => {
            let created = sent(
                client
                    .post(&endpoint)
                    .bearer_auth(&token)
                    .json(&serde_json::json!({ "name": name, "tables": tables })),
            )
            .await?;

            (created, None)
        }
    };

    let id = answer
        .get("id")
        .and_then(Json::as_str)
        .map(str::to_string)
        .or(requested)
        .ok_or_else(|| "the site did not hand back a room".to_string())?;

    Ok(SharedErd {
        link: format!("{}/erd/{id}", site.trim_end_matches('/')),
        open: answer.get("open").and_then(Json::as_bool).unwrap_or(false),
        id,
    })
}

#[tauri::command]
async fn share_erd(site: String, id: String, open: bool) -> Result<bool, String> {
    let token = signed_in().await?;
    let answer = sent(
        reqwest::Client::new()
            .patch(erd_endpoint(&site))
            .bearer_auth(token)
            .json(&serde_json::json!({ "id": id, "open": open })),
    )
    .await?;

    answer
        .get("open")
        .and_then(Json::as_bool)
        .ok_or_else(|| "the site did not say whether the room is open".to_string())
}

#[tauri::command]
async fn close_erd(site: String, id: String) -> Result<(), String> {
    let token = signed_in().await?;

    sent(
        reqwest::Client::new()
            .delete(erd_endpoint(&site))
            .bearer_auth(token)
            .json(&serde_json::json!({ "id": id })),
    )
    .await
    .map(|_| ())
}

#[tauri::command]
async fn list_erd(site: String) -> Result<Vec<ErdRoom>, String> {
    let token = signed_in().await?;
    let answer = sent(
        reqwest::Client::new()
            .get(erd_endpoint(&site))
            .bearer_auth(token),
    )
    .await?;

    Ok(answer
        .get("rooms")
        .and_then(Json::as_array)
        .map(|rooms| rooms.iter().filter_map(ErdRoom::read).collect())
        .unwrap_or_default())
}

#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    let link = url::Url::parse(url.trim())
        .map_err(|error| format!("that link cannot be read: {error}"))?;

    if !matches!(link.scheme(), "http" | "https" | "mailto") {
        return Err(format!("{} links are not opened from GPQL", link.scheme()));
    }

    tauri_plugin_opener::open_url(link.as_str(), None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
fn port_free(port: u16) -> bool {
    tunnel::free(port)
}

#[tauri::command]
async fn tailnet_peers() -> Result<Vec<tailnet::Peer>, String> {
    tailnet::peers().await
}

#[tauri::command]
fn backends() -> &'static [backends::Backend] {
    backends::CATALOG
}

#[tauri::command]
fn credentials() -> Result<Vec<Credential>, String> {
    vault::credentials()
}

#[tauri::command]
fn save_credential(credential: Credential) -> Result<(), String> {
    vault::save_credential(credential)
}

#[tauri::command]
fn forget_credential(name: String) -> Result<(), String> {
    vault::forget_credential(&name)
}

#[tauri::command]
fn providers() -> Result<Vec<Provider>, String> {
    vault::providers()
}

#[tauri::command]
fn save_provider(provider: Provider) -> Result<(), String> {
    vault::save_provider(provider)
}

#[tauri::command]
fn forget_provider(id: String) -> Result<(), String> {
    vault::forget_provider(&id)
}

#[tauri::command]
async fn save_connection(config: SessionConfig) -> Result<String, String> {
    let described = vault::describe(&config);

    off_thread(move || vault::remember(&config)).await?;

    Ok(described)
}

#[derive(serde::Serialize)]
struct Release {
    current: String,
    latest: String,
    link: String,
    fresh: bool,
}

#[tauri::command]
async fn latest_release() -> Result<Release, String> {
    let response = reqwest::Client::new()
        .get("https://api.github.com/repos/pleahmacaka/gpql/releases/latest")
        .header("user-agent", "gpql")
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("no release published yet".into());
    }

    let answer = net::answer(response).await?;

    let latest = answer
        .get("tag_name")
        .and_then(|tag| tag.as_str())
        .ok_or("GitHub answered without a release tag")?
        .trim_start_matches('v')
        .to_string();

    let link = answer
        .get("html_url")
        .and_then(|url| url.as_str())
        .unwrap_or("https://github.com/pleahmacaka/gpql/releases/latest")
        .to_string();

    let current = env!("CARGO_PKG_VERSION").to_string();
    let fresh = older(&current, &latest);

    Ok(Release {
        current,
        latest,
        link,
        fresh,
    })
}

fn older(current: &str, latest: &str) -> bool {
    let parts = |text: &str| -> Vec<u32> {
        text.split('.')
            .map(|piece| piece.parse::<u32>().unwrap_or(0))
            .collect()
    };

    let (mine, theirs) = (parts(current), parts(latest));

    for at in 0..mine.len().max(theirs.len()) {
        let a = mine.get(at).copied().unwrap_or(0);
        let b = theirs.get(at).copied().unwrap_or(0);

        if a != b {
            return a < b;
        }
    }

    false
}

#[tauri::command]
async fn openrouter_models() -> Result<Vec<String>, String> {
    let answer = sent(reqwest::Client::new().get("https://openrouter.ai/api/v1/models")).await?;

    let mut names: Vec<String> = answer
        .get("data")
        .and_then(|data| data.as_array())
        .map(|models| {
            models
                .iter()
                .filter_map(|model| model.get("id").and_then(|id| id.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    names.sort();

    Ok(names)
}

#[tauri::command]
async fn connect_openrouter(model: String) -> Result<Provider, String> {
    let key = login::openrouter().await?;
    let provider = Provider {
        id: "openrouter".into(),
        name: "OpenRouter".into(),
        base_url: "https://openrouter.ai/api/v1".into(),
        model: if model.is_empty() {
            "openai/gpt-4o-mini".into()
        } else {
            model
        },
        key,
    };

    let saving = provider.clone();

    off_thread(move || vault::save_provider(saving)).await?;

    Ok(provider)
}

#[tauri::command]
async fn sign_in(site: String) -> Result<(), String> {
    let token = login::sign_in(&site).await?;

    off_thread(move || vault::set_account_token(&token)).await
}

#[tauri::command]
fn saved_logins() -> Result<Vec<vault::ListedLogin>, String> {
    vault::listed()
}

#[tauri::command]
fn saved_login(url: String) -> Result<Option<SavedLogin>, String> {
    vault::find(&url)
}

#[tauri::command]
fn saved_logins_moved() -> Option<String> {
    vault::set_aside_notice()
}

#[tauri::command]
fn forget_login(url: String) -> Result<(), String> {
    vault::forget(&url)
}

#[tauri::command]
fn forget_all_logins() -> Result<(), String> {
    vault::forget_all()
}

#[tauri::command]
fn logins_location() -> String {
    vault::logins_path()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

// openssh writes the public half beside the private one and keeps its own
// bookkeeping in the same folder, so neither belongs in the list
fn keys_in(folder: &std::path::Path) -> Vec<String> {
    const NOT_KEYS: [&str; 4] = [
        "config",
        "known_hosts",
        "known_hosts.old",
        "authorized_keys",
    ];

    let Ok(listing) = std::fs::read_dir(folder) else {
        return Vec::new();
    };

    let mut found: Vec<String> = listing
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();

            !name.ends_with(".pub") && !NOT_KEYS.contains(&name.as_str())
        })
        .map(|entry| entry.path().display().to_string())
        .collect();

    found.sort();

    found
}

#[tauri::command]
fn ssh_keys() -> Vec<String> {
    let Some(folder) = dirs::home_dir().map(|home| home.join(".ssh")) else {
        return Vec::new();
    };

    keys_in(&folder)
}

#[cfg(test)]
mod ssh_folder {
    use super::*;

    #[test]
    fn only_private_keys_are_offered() {
        let folder = std::env::temp_dir().join("gpql_ssh_probe");

        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();

        for name in [
            "id_ed25519",
            "id_ed25519.pub",
            "id_rsa",
            "known_hosts",
            "config",
            "authorized_keys",
        ] {
            std::fs::write(folder.join(name), "x").unwrap();
        }

        let named: Vec<String> = keys_in(&folder)
            .into_iter()
            .map(|path| {
                std::path::Path::new(&path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert_eq!(named, ["id_ed25519", "id_rsa"]);

        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_missing_folder_is_not_an_error() {
        assert!(keys_in(std::path::Path::new("no such folder here")).is_empty());
    }
}

#[tauri::command]
fn account_token() -> Option<String> {
    vault::account_token()
}

#[tauri::command]
fn set_account_token(token: String) -> Result<(), String> {
    vault::set_account_token(&token)
}

#[tauri::command]
fn forget_account() -> Result<(), String> {
    vault::clear_account()
}

#[tauri::command]
fn local_query(
    sql: String,
    params: Vec<Json>,
    store: State<'_, Local>,
) -> Result<Vec<Vec<Json>>, String> {
    local::run(&store, &sql, &params)
}

#[tauri::command]
fn local_batch(sql: String, store: State<'_, Local>) -> Result<(), String> {
    local::batch(&store, &sql)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Sessions::default())
        .manage(Tunnels::default())
        .manage(Local::open().expect("gpql could not open its local database"))
        .manage(Highlighter::new().expect("gpql could not load its SQL grammar"))
        .manage(Servers::default())
        .manage(Busy::default())
        .invoke_handler(tauri::generate_handler![
            check,
            probe_recents,
            save_connection,
            databases,
            connect,
            disconnect,
            reset_sessions,
            objects,
            object_ddl,
            mqtt_publish,
            mqtt_clear,
            mqtt_subscribe,
            mqtt_unsubscribe,
            mqtt_subscriptions,
            s3_presign,
            s3_download,
            s3_upload,
            s3_delete,
            s3_refresh,
            explain_query,
            export_table,
            export_result,
            tables,
            schemas,
            use_schema,
            table_rows,
            built_query,
            set_read_only,
            set_manual,
            end_transaction,
            pending_edits,
            run_query,
            cancel_query,
            schema,
            apply_edits,
            highlight_sql,
            check_sql,
            lsp_start,
            lsp_stop,
            lsp_running,
            lsp_complete,
            set_acrylic,
            read_document,
            write_document,
            look_on_this_machine,
            scan_local,
            scan_tailnet,
            tailnet_peers,
            port_free,
            publish_schema,
            share_erd,
            close_erd,
            list_erd,
            open_link,
            backends,
            credentials,
            save_credential,
            forget_credential,
            sign_in,
            providers,
            save_provider,
            forget_provider,
            connect_openrouter,
            openrouter_models,
            latest_release,
            saved_logins,
            saved_login,
            saved_logins_moved,
            forget_login,
            forget_all_logins,
            logins_location,
            account_token,
            ssh_keys,
            set_account_token,
            forget_account,
            local_query,
            local_batch,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            #[cfg(all(debug_assertions, windows))]
            {
                use tauri::Manager;

                let mut config = app.config().app.windows[0].clone();
                config.label = "debug".to_string();

                let debug_window = tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?
                    .data_directory(std::env::temp_dir().join("gpql-debug-webview"))
                    .additional_browser_args(
                        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
                         --autoplay-policy=no-user-gesture-required \
                         --remote-debugging-port=9222",
                    )
                    .build()?;

                if let Some(main) = app.get_webview_window("main") {
                    main.close()?;
                }

                debug_window.set_focus()?;
            }

            Ok(())
        });

    #[cfg(debug_assertions)]
    let builder = builder.plugin(
        tauri_plugin_mcp_bridge::Builder::new()
            .bind_address("127.0.0.1")
            .build(),
    );

    let mut context = tauri::generate_context!();

    // the debug-only mcp bridge injects inline scripts, which the shipped csp refuses
    if cfg!(debug_assertions) {
        context.config_mut().app.security.csp = None;
    }

    builder
        .run(context)
        .expect("error while running tauri application");
}

#[cfg(test)]
mod waypoints {
    use super::*;

    #[test]
    fn reads_addresses() {
        assert_eq!(
            address_of("https://eu-1.turso.io"),
            Some(("eu-1.turso.io".into(), 443))
        );
        assert_eq!(
            address_of("http://127.0.0.1:8086"),
            Some(("127.0.0.1".into(), 8086))
        );
        assert_eq!(
            address_of("http://box:8123/metrics"),
            Some(("box".into(), 8123))
        );
        assert_eq!(address_of("db.host:5432"), Some(("db.host".into(), 5432)));
    }

    #[test]
    fn spots_missing_files() {
        assert_eq!(missing_file("Cargo.toml"), "");
        assert_eq!(missing_file("no-such-file.db"), "gone");
    }
}
