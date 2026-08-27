use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use serde::Serialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, oneshot};
use tokio::time::{Duration, timeout};

const DOCUMENT: &str = "file:///gpql/query";
const PATIENCE: Duration = Duration::from_millis(1500);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Completion {
    pub label: String,
    pub detail: String,
    pub kind: i64,
}

struct Server {
    child: Child,
    stdin: ChildStdin,
    next: AtomicI64,
    version: AtomicI64,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
}

#[derive(Default)]
pub struct Servers {
    running: Mutex<HashMap<String, Server>>,
}

impl Servers {
    pub async fn start(
        &self,
        dialect: &str,
        program: &str,
        args: &[String],
    ) -> Result<(), String> {
        self.stop(dialect).await;

        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        #[cfg(windows)]
        command.creation_flags(0x0800_0000);

        let mut child = command.spawn().map_err(|error| error.to_string())?;
        let stdin = child.stdin.take().ok_or("the server took no input")?;
        let stdout = child.stdout.take().ok_or("the server gave no output")?;

        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        listen(stdout, pending.clone());

        let mut server = Server {
            child,
            stdin,
            next: AtomicI64::new(1),
            version: AtomicI64::new(1),
            pending,
        };

        server
            .request(
                "initialize",
                json!({
                    "processId": std::process::id(),
                    "rootUri": Value::Null,
                    "capabilities": {
                        "textDocument": {
                            "completion": { "completionItem": { "snippetSupport": false } },
                        }
                    },
                }),
            )
            .await?;

        server.notify("initialized", json!({})).await?;
        server
            .notify(
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": DOCUMENT,
                        "languageId": dialect,
                        "version": 1,
                        "text": "",
                    }
                }),
            )
            .await?;

        self.running.lock().await.insert(dialect.to_string(), server);

        return Ok(());
    }

    pub async fn stop(&self, dialect: &str) {
        if let Some(mut server) = self.running.lock().await.remove(dialect) {
            let _ = server.child.kill().await;
        }
    }

    pub async fn running(&self) -> Vec<String> {
        return self.running.lock().await.keys().cloned().collect();
    }

    pub async fn sync(&self, dialect: &str, text: &str) -> Result<(), String> {
        let mut running = self.running.lock().await;
        let Some(server) = running.get_mut(dialect) else {
            return Err("no language server for that dialect".into());
        };

        let version = server.version.fetch_add(1, Ordering::Relaxed) + 1;

        return server
            .notify(
                "textDocument/didChange",
                json!({
                    "textDocument": { "uri": DOCUMENT, "version": version },
                    "contentChanges": [{ "text": text }],
                }),
            )
            .await;
    }

    pub async fn complete(
        &self,
        dialect: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Completion>, String> {
        let mut running = self.running.lock().await;
        let Some(server) = running.get_mut(dialect) else {
            return Ok(Vec::new());
        };

        let answer = server
            .request(
                "textDocument/completion",
                json!({
                    "textDocument": { "uri": DOCUMENT },
                    "position": { "line": line, "character": character },
                }),
            )
            .await?;

        let items = answer
            .get("items")
            .or(Some(&answer))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        return Ok(items
            .iter()
            .filter_map(|item| {
                Some(Completion {
                    label: item.get("label")?.as_str()?.to_string(),
                    detail: item
                        .get("detail")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    kind: item.get("kind").and_then(Value::as_i64).unwrap_or(1),
                })
            })
            .take(50)
            .collect());
    }
}

impl Server {
    async fn send(&mut self, payload: Value) -> Result<(), String> {
        let body = payload.to_string();
        let framed = format!("Content-Length: {}\r\n\r\n{body}", body.len());

        return self
            .stdin
            .write_all(framed.as_bytes())
            .await
            .map_err(|error| error.to_string());
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        return self
            .send(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
            .await;
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();

        self.pending.lock().await.insert(id, sender);
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await?;

        let answer = timeout(PATIENCE, receiver)
            .await
            .map_err(|_| format!("{method} timed out"))?
            .map_err(|_| "the language server went away".to_string())?;

        if let Some(message) = answer.pointer("/error/message").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        return Ok(answer.get("result").cloned().unwrap_or(Value::Null));
    }
}

fn listen(
    stdout: tokio::process::ChildStdout,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);

        loop {
            let mut length = 0usize;

            loop {
                let mut header = String::new();

                if reader.read_line(&mut header).await.unwrap_or(0) == 0 {
                    return;
                }

                let trimmed = header.trim();

                if trimmed.is_empty() {
                    break;
                }

                if let Some(value) = trimmed.strip_prefix("Content-Length:") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }

            if length == 0 {
                continue;
            }

            let mut body = vec![0u8; length];

            if reader.read_exact(&mut body).await.is_err() {
                return;
            }

            let Ok(message): Result<Value, _> = serde_json::from_slice(&body) else {
                continue;
            };

            if let Some(id) = message.get("id").and_then(Value::as_i64) {
                if let Some(sender) = pending.lock().await.remove(&id) {
                    let _ = sender.send(message);
                }

                continue;
            }
        }
    });
}

