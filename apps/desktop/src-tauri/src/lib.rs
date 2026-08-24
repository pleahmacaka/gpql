mod acp;
mod ai;
mod backends;
mod db;
mod duck;
mod highlight;
mod local;
mod login;
mod lsp;
mod mysql;
mod remote;
mod tailnet;
mod vault;

use db::{
    Discovery, QueryResult, SessionConfig, SessionHandle, Sessions, TableInfo, TableSchema,
};
use highlight::{Highlighter, Token};
use acp::Assistant;
use local::Local;
use lsp::{Completion, Diagnostic, Servers};
use serde_json::Value as Json;
use tauri::State;
use vault::{Credential, Provider, SavedLogin};

#[tauri::command]
async fn check(config: SessionConfig) -> Result<String, String> {
    let session = db::open(&config).await?;
    let probe = if config.kind == "sqlite" {
        "select 'sqlite ' || sqlite_version()"
    } else {
        "select version()"
    };

    let result = db::query(&session, probe).await?;

    return Ok(result.rows[0][0].clone().unwrap_or_else(|| "reachable".into()));
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
async fn agent_start(
    program: String,
    args: Vec<String>,
    assistant: State<'_, Assistant>,
) -> Result<(), String> {
    return assistant.start(&program, &args).await;
}

#[tauri::command]
async fn agent_stop(assistant: State<'_, Assistant>) -> Result<(), String> {
    assistant.stop().await;

    return Ok(());
}

#[tauri::command]
async fn agent_ready(assistant: State<'_, Assistant>) -> Result<bool, String> {
    return Ok(assistant.ready().await);
}

#[tauri::command]
async fn agent_chat(
    prompt: String,
    assistant: State<'_, Assistant>,
) -> Result<String, String> {
    return assistant.ask(&prompt).await;
}

#[tauri::command]
async fn agent_sql(
    prompt: String,
    session_id: String,
    assistant: State<'_, Assistant>,
    sessions: State<'_, Sessions>,
) -> Result<String, String> {
    let outline = match sessions.get(&session_id) {
        Ok(session) => ai::outline(&db::schema(&session).await?),
        Err(_) => String::new(),
    };

    let ask = format!(
        "Write one SQL statement for this schema. Answer with SQL only, no prose,          no code fences.

Schema:
{outline}

Request: {prompt}"
    );

    return assistant.ask(&ask).await;
}

#[tauri::command]
async fn ask_sql(
    provider_id: String,
    prompt: String,
    session_id: String,
    sessions: State<'_, Sessions>,
) -> Result<String, String> {
    let provider = vault::providers()
        .into_iter()
        .find(|entry| entry.id == provider_id)
        .ok_or_else(|| "that provider is gone".to_string())?;

    let outline = match sessions.get(&session_id) {
        Ok(session) => ai::outline(&db::schema(&session).await?),
        Err(_) => String::new(),
    };

    return ai::write_sql(&provider, &prompt, &outline).await;
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
        .manage(Assistant::default())
        .invoke_handler(tauri::generate_handler![
            check,
            databases,
            connect,
            disconnect,
            tables,
            table_rows,
            run_query,
            schema,
            highlight_sql,
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
            ask_sql,
            agent_start,
            agent_stop,
            agent_ready,
            agent_chat,
            agent_sql,
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
