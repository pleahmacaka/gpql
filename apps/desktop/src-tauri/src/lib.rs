mod editor;
mod engines;
mod net;
mod store;

use crate::engines::backends;
use crate::engines::db;
use crate::editor::highlight;
use crate::editor::lsp;
use crate::store::local;
use crate::store::vault;
use crate::net::login;
use crate::net::tailnet;

use db::{
    Discovery, Edit, QueryResult, SessionConfig, SessionHandle, Sessions, TableInfo,
    TableSchema,
};
use highlight::{Highlighter, Token};
use local::Local;
use lsp::{Completion, Diagnostic, Servers};
use serde_json::Value as Json;
use tauri::State;
use vault::{Credential, Provider, SavedLogin};

#[tauri::command]
async fn check(config: SessionConfig) -> Result<String, String> {
    let session = db::open(&config).await?;

    let probe = match (config.kind.as_str(), backends::dialect_of(&config.kind)) {
        ("sqlite", _) => "select 'sqlite ' || sqlite_version()",
        (_, "cypher") => "return 1",
        (_, "flux") => {
            let listed = db::tables(&session).await?;

            return Ok(format!("{} buckets", listed.len()));
        }
        ("clickhouse", _) => "select version()",
        ("snowflake", _) => "select current_version()",
        ("influxdb", _) => "select 1",
        ("turso" | "d1", _) => "select sqlite_version()",
        _ => "select version()",
    };

    let result = db::query(&session, probe).await?;

    return Ok(result
        .rows
        .first()
        .and_then(|row| row.first().cloned().flatten())
        .unwrap_or_else(|| "reachable".into()));
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Waypoint {
    url: String,
    kind: String,
}

#[tauri::command]
async fn probe_recents(items: Vec<Waypoint>) -> Vec<String> {
    let logins = vault::list();

    let checks = items.into_iter().map(|item| {
        if item.kind == "erd" {
            return Some(Look::File(item.url));
        }

        let login = logins
            .iter()
            .find(|saved| saved.url == item.url)
            .or_else(|| logins.iter().find(|saved| tail(&saved.url) == tail(&item.url)))?;

        if !login.path.is_empty() {
            return Some(Look::File(login.path.clone()));
        }

        if login.endpoint.is_empty() {
            return address_of(&format!(
                "{}:{}",
                login.host,
                if login.port.is_empty() { "5432" } else { &login.port }
            ))
            .map(|(host, port)| Look::Port(host, port));
        }

        return address_of(&login.endpoint).map(|(host, port)| Look::Port(host, port));
    });

    let answers = checks.map(|check| {
        tokio::task::spawn_blocking(move || match check {
            None => String::new(),
            Some(Look::File(path)) => missing_file(&path),
            Some(Look::Port(host, port)) if host.is_empty() => {
                let _ = port;

                String::new()
            }
            Some(Look::Port(host, port)) => {
                if db::reachable(&host, port, 400) {
                    String::new()
                } else {
                    "down".to_string()
                }
            }
        })
    });

    let mut out = Vec::new();

    for answer in answers.collect::<Vec<_>>() {
        out.push(answer.await.unwrap_or_default());
    }

    return out;
}

enum Look {
    File(String),
    Port(String, u16),
}

fn tail(url: &str) -> &str {
    return match url.find("://") {
        Some(at) => &url[at + 3..],
        None => url,
    };
}

fn missing_file(path: &str) -> String {
    if std::path::Path::new(path).exists() {
        return String::new();
    }

    return "gone".to_string();
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

    return Some((host.to_string(), if secure { 443 } else { 80 }));
}

#[tauri::command]
async fn databases(config: SessionConfig) -> Result<Vec<String>, String> {
    use crate::backends::Transport;

    if config.kind == "supabase_api" {
        return supabase_projects(&config.token).await;
    }

    let listing = match crate::backends::transport_of(&config.kind) {
        Transport::Postgres => "select datname from pg_database where datistemplate = false order by 1",
        Transport::MySql => "show databases",
        _ => return Ok(Vec::new()),
    };

    let mut probing = config.clone();

    if probing.database.is_empty() {
        probing.database = "postgres".into();
    }

    probing.read_only = true;

    let session = db::open(&probing).await?;
    let result = db::query(&session, listing).await?;

    return Ok(result
        .rows
        .into_iter()
        .filter_map(|row| row.into_iter().next().flatten())
        .collect());
}

async fn supabase_projects(token: &str) -> Result<Vec<String>, String> {
    if token.is_empty() {
        return Ok(Vec::new());
    }

    let body: Json = reqwest::Client::new()
        .get("https://api.supabase.com/v1/projects")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    if let Some(message) = body.get("message").and_then(Json::as_str) {
        return Err(message.to_string());
    }

    return Ok(body
        .as_array()
        .map(|projects| {
            projects
                .iter()
                .filter_map(|project| project.get("id").and_then(Json::as_str))
                .map(|id| id.to_string())
                .collect()
        })
        .unwrap_or_default());
}

#[tauri::command]
async fn connect(
    config: SessionConfig,
    sessions: State<'_, Sessions>,
) -> Result<SessionHandle, String> {
    let session = db::open(&config).await?;
    vault::remember(&config)?;

    return Ok(sessions.insert(session));
}

#[tauri::command]
fn disconnect(id: String, sessions: State<'_, Sessions>) {
    sessions.remove(&id);
}

#[tauri::command]
async fn tables(id: String, sessions: State<'_, Sessions>) -> Result<Vec<TableInfo>, String> {
    let session = sessions.get(&id)?;

    return db::tables(&session).await;
}

#[tauri::command]
fn check_sql(
    sql: String,
    dialect: String,
    reader: State<'_, highlight::Highlighter>,
) -> Option<highlight::Fault> {
    return reader.fault(&dialect, &sql);
}

#[tauri::command]
async fn set_read_only(
    id: String,
    on: bool,
    sessions: State<'_, Sessions>,
) -> Result<(), String> {
    let session = sessions.get(&id)?;

    return db::set_read_only(&session, on).await;
}

#[tauri::command]
async fn table_rows(
    id: String,
    table: String,
    limit: u32,
    offset: u32,
    sessions: State<'_, Sessions>,
) -> Result<QueryResult, String> {
    let session = sessions.get(&id)?;

    return db::table_rows(&session, &table, limit, offset).await;
}

#[tauri::command]
async fn run_query(
    id: String,
    sql: String,
    sessions: State<'_, Sessions>,
) -> Result<QueryResult, String> {
    let session = sessions.get(&id)?;

    return db::query(&session, &sql).await;
}

#[tauri::command]
async fn apply_edits(
    id: String,
    table: String,
    edits: Vec<Edit>,
    sessions: State<'_, Sessions>,
) -> Result<u64, String> {
    let session = sessions.get(&id)?;

    return db::apply(&session, &table, &edits).await;
}

#[tauri::command]
async fn schema(id: String, sessions: State<'_, Sessions>) -> Result<Vec<TableSchema>, String> {
    let session = sessions.get(&id)?;

    return db::schema(&session).await;
}

#[tauri::command]
fn highlight_sql(
    sql: String,
    dialect: String,
    highlighter: State<'_, Highlighter>,
) -> Vec<Token> {
    return highlighter.tokens(&dialect, &sql);
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

    return Ok(());
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

#[tauri::command]
async fn lsp_diagnostics(
    dialect: String,
    servers: State<'_, Servers>,
) -> Result<Vec<Diagnostic>, String> {
    return Ok(servers.diagnostics(&dialect).await);
}

#[tauri::command]
fn read_document(path: String) -> Result<String, String> {
    return std::fs::read_to_string(&path).map_err(|e| e.to_string());
}

#[tauri::command]
fn write_document(path: String, text: String) -> Result<(), String> {
    return std::fs::write(&path, text).map_err(|e| e.to_string());
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

    return window.set_effects(effects).map_err(|e| e.to_string());
}

#[tauri::command]
fn look_on_this_machine() -> Vec<u16> {
    return db::local_postgres_ports();
}

fn probe_credentials() -> Vec<(String, String)> {
    let mut candidates: Vec<(String, String)> = vault::list()
        .into_iter()
        .filter(|login| login.kind == "postgres")
        .map(|login| (login.user, login.password))
        .collect();

    candidates.extend(
        vault::credentials()
            .into_iter()
            .map(|preset| (preset.user, preset.password)),
    );
    candidates.dedup();

    return candidates;
}

#[tauri::command]
async fn scan_local() -> Vec<Discovery> {
    return db::scan(&probe_credentials()).await;
}

#[tauri::command]
async fn scan_tailnet() -> Vec<Discovery> {
    let candidates = probe_credentials();
    let mut found = Vec::new();

    for peer in tailnet::peers().into_iter().filter(|peer| peer.online) {
        found.extend(db::scan_host(&peer.host, &[5432, 5433], &candidates).await);
    }

    return found;
}

#[tauri::command]
async fn publish_schema(
    site: String,
    name: String,
    sessions: State<'_, Sessions>,
    session_id: String,
) -> Result<String, String> {
    let token = vault::account_token().ok_or_else(|| "sign in first".to_string())?;
    let session = sessions.get(&session_id)?;
    let tables = db::schema(&session).await?;

    let answer: serde_json::Value = reqwest::Client::new()
        .post(format!("{}/api/erd", site.trim_end_matches('/')))
        .bearer_auth(token)
        .json(&serde_json::json!({ "name": name, "tables": tables }))
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    let id = answer
        .get("id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "the site did not hand back a room".to_string())?;

    let link = format!("{}/erd/{id}", site.trim_end_matches('/'));

    tauri_plugin_opener::open_url(&link, None::<&str>).map_err(|e| e.to_string())?;

    return Ok(link);
}

#[tauri::command]
fn tailnet_peers() -> Vec<tailnet::Peer> {
    return tailnet::peers();
}

#[tauri::command]
fn backends() -> &'static [backends::Backend] {
    return backends::CATALOG;
}

#[tauri::command]
fn credentials() -> Vec<Credential> {
    return vault::credentials();
}

#[tauri::command]
fn save_credential(credential: Credential) -> Result<(), String> {
    return vault::save_credential(credential);
}

#[tauri::command]
fn forget_credential(name: String) -> Result<(), String> {
    return vault::forget_credential(&name);
}

#[tauri::command]
fn providers() -> Vec<Provider> {
    return vault::providers();
}

#[tauri::command]
fn save_provider(provider: Provider) -> Result<(), String> {
    return vault::save_provider(provider);
}

#[tauri::command]
fn forget_provider(id: String) -> Result<(), String> {
    return vault::forget_provider(&id);
}

#[tauri::command]
async fn save_connection(config: SessionConfig) -> Result<String, String> {
    vault::remember(&config)?;

    return Ok(vault::describe(&config));
}

#[tauri::command]
async fn openrouter_models() -> Result<Vec<String>, String> {
    let answer: serde_json::Value = reqwest::Client::new()
        .get("https://openrouter.ai/api/v1/models")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

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

    return Ok(names);
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

    vault::save_provider(provider.clone())?;

    return Ok(provider);
}

#[tauri::command]
async fn sign_in(site: String) -> Result<(), String> {
    let token = login::sign_in(&site).await?;

    return vault::set_account_token(&token);
}

#[tauri::command]
fn saved_logins() -> Vec<SavedLogin> {
    return vault::list();
}

#[tauri::command]
fn forget_login(url: String) -> Result<(), String> {
    return vault::forget(&url);
}

#[tauri::command]
fn forget_all_logins() -> Result<(), String> {
    return vault::forget_all();
}

#[tauri::command]
fn logins_location() -> String {
    return vault::logins_path()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
}

#[tauri::command]
fn account_token() -> Option<String> {
    return vault::account_token();
}

#[tauri::command]
fn set_account_token(token: String) -> Result<(), String> {
    return vault::set_account_token(&token);
}

#[tauri::command]
fn forget_account() -> Result<(), String> {
    return vault::clear_account();
}

#[tauri::command]
fn local_query(sql: String, params: Vec<Json>, store: State<'_, Local>) -> Result<Vec<Vec<Json>>, String> {
    return local::run(&store, &sql, &params);
}

#[tauri::command]
fn local_batch(sql: String, store: State<'_, Local>) -> Result<(), String> {
    return local::batch(&store, &sql);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Sessions::default())
        .manage(Local::open().expect("gpql could not open its local database"))
        .manage(Highlighter::new().expect("gpql could not load its SQL grammar"))
        .manage(Servers::default())
        .invoke_handler(tauri::generate_handler![
            check,
            probe_recents,
            save_connection,
            databases,
            connect,
            disconnect,
            tables,
            table_rows,
            set_read_only,
            run_query,
            schema,
            apply_edits,
            highlight_sql,
            check_sql,
            lsp_start,
            lsp_stop,
            lsp_running,
            lsp_complete,
            lsp_diagnostics,
            set_acrylic,
            read_document,
            write_document,
            look_on_this_machine,
            scan_local,
            scan_tailnet,
            tailnet_peers,
            publish_schema,
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
            saved_logins,
            forget_login,
            forget_all_logins,
            logins_location,
            account_token,
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

            Ok(())
        })
        .run(tauri::generate_context!())
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
