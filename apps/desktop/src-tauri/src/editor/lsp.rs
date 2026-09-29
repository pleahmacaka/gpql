use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{timeout, Duration};

const DOCUMENT: &str = "file:///gpql/query";
const WAKING: Duration = Duration::from_secs(10);
const PATIENCE: Duration = Duration::from_millis(1500);
const LARGEST_MESSAGE: usize = 64 * 1024 * 1024;

type Pending = Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Completion {
    pub label: String,
    pub detail: String,
    pub kind: i64,
}

type Process = Arc<std::sync::Mutex<Child>>;

struct Server {
    child: Process,
    outbox: mpsc::Sender<Value>,
    next: AtomicI64,
    version: AtomicI64,
    pending: Pending,
}

#[derive(Default)]
pub struct Servers {
    running: Mutex<HashMap<String, Arc<Server>>>,
}

fn launchable(program: &str) -> Result<&str, String> {
    let program = program.trim();

    if program.is_empty() || program.chars().any(char::is_control) {
        return Err("name the language server program to start".into());
    }

    let path = Path::new(program);

    if !path.is_absolute() && path.components().count() > 1 {
        return Err(format!(
            "{program} is a relative path; give its full path or a program name on PATH"
        ));
    }

    Ok(program)
}

impl Servers {
    pub async fn start(&self, dialect: &str, program: &str, args: &[String]) -> Result<(), String> {
        let program = launchable(program)?;

        self.stop(dialect).await;

        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        #[cfg(windows)]
        command.creation_flags(0x0800_0000);

        let mut child = command
            .spawn()
            .map_err(|error| format!("{program}: {error}"))?;
        let stdin = child.stdin.take().ok_or("the server took no input")?;
        let stdout = child.stdout.take().ok_or("the server gave no output")?;

        let pending = Pending::default();
        let (outbox, queue) = mpsc::channel(64);
        let child: Process = Arc::new(std::sync::Mutex::new(child));

        write(stdin, queue);
        listen(stdout, pending.clone(), outbox.clone(), Arc::clone(&child));

        let server = Server {
            child,
            outbox,
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
                WAKING,
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

        let replaced = self
            .running
            .lock()
            .await
            .insert(dialect.to_string(), Arc::new(server));

        if let Some(replaced) = replaced {
            replaced.kill();
        }

        Ok(())
    }

    pub async fn stop(&self, dialect: &str) {
        let removed = self.running.lock().await.remove(dialect);

        if let Some(server) = removed {
            server.kill();
        }
    }

    pub async fn running(&self) -> Vec<String> {
        self.running.lock().await.keys().cloned().collect()
    }

    async fn get(&self, dialect: &str) -> Option<Arc<Server>> {
        self.running.lock().await.get(dialect).cloned()
    }

    pub async fn sync(&self, dialect: &str, text: &str) -> Result<(), String> {
        let server = self
            .get(dialect)
            .await
            .ok_or("no language server for that dialect")?;

        let version = server.version.fetch_add(1, Ordering::Relaxed) + 1;

        server
            .notify(
                "textDocument/didChange",
                json!({
                    "textDocument": { "uri": DOCUMENT, "version": version },
                    "contentChanges": [{ "text": text }],
                }),
            )
            .await
    }

    pub async fn complete(
        &self,
        dialect: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Completion>, String> {
        let Some(server) = self.get(dialect).await else {
            return Ok(Vec::new());
        };

        let answer = server
            .request(
                "textDocument/completion",
                json!({
                    "textDocument": { "uri": DOCUMENT },
                    "position": { "line": line, "character": character },
                }),
                PATIENCE,
            )
            .await?;

        let items = answer
            .get("items")
            .or(Some(&answer))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        Ok(items
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
            .collect())
    }
}

fn kill(child: &Process) {
    if let Ok(mut child) = child.lock() {
        let _ = child.start_kill();
    }
}

impl Server {
    fn kill(&self) {
        kill(&self.child);
    }

    async fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        let message = json!({ "jsonrpc": "2.0", "method": method, "params": params });

        timeout(PATIENCE, self.outbox.send(message))
            .await
            .map_err(|_| format!("{method} timed out"))?
            .map_err(|_| "the language server went away".to_string())
    }

    async fn request(
        &self,
        method: &str,
        params: Value,
        patience: Duration,
    ) -> Result<Value, String> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();

        self.pending.lock().await.insert(id, sender);

        let asked = async {
            self.outbox
                .send(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": method,
                    "params": params,
                }))
                .await
                .map_err(|_| "the language server went away".to_string())?;

            receiver
                .await
                .map_err(|_| "the language server went away".to_string())
        };

        let answer = timeout(patience, asked).await;

        if !matches!(answer, Ok(Ok(_))) {
            self.pending.lock().await.remove(&id);
        }

        let answer = answer.map_err(|_| format!("{method} timed out"))??;

        if let Some(message) = answer.pointer("/error/message").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        Ok(answer.get("result").cloned().unwrap_or(Value::Null))
    }
}

fn write(mut stdin: ChildStdin, mut queue: mpsc::Receiver<Value>) {
    tokio::spawn(async move {
        while let Some(payload) = queue.recv().await {
            let body = payload.to_string();
            let framed = format!("Content-Length: {}\r\n\r\n{body}", body.len());

            if stdin.write_all(framed.as_bytes()).await.is_err() {
                return;
            }
        }
    });
}

fn answer_to(method: &str, id: Value, params: Option<&Value>) -> Value {
    match method {
        "workspace/configuration" => {
            let asked = params
                .and_then(|params| params.get("items"))
                .and_then(Value::as_array)
                .map_or(0, Vec::len);

            json!({ "jsonrpc": "2.0", "id": id, "result": vec![Value::Null; asked] })
        }
        "client/registerCapability"
        | "client/unregisterCapability"
        | "window/workDoneProgress/create"
        | "window/showMessageRequest" => json!({ "jsonrpc": "2.0", "id": id, "result": null }),
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("gpql does not handle {method}") },
        }),
    }
}

fn listen(stdout: ChildStdout, pending: Pending, outbox: mpsc::Sender<Value>, child: Process) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);

        loop {
            let mut length = 0usize;

            loop {
                let mut header = String::new();

                if reader.read_line(&mut header).await.unwrap_or(0) == 0 {
                    pending.lock().await.clear();

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

            if length > LARGEST_MESSAGE {
                pending.lock().await.clear();
                kill(&child);

                return;
            }

            let mut body = vec![0u8; length];

            if reader.read_exact(&mut body).await.is_err() {
                pending.lock().await.clear();

                return;
            }

            let Ok(message): Result<Value, _> = serde_json::from_slice(&body) else {
                continue;
            };

            if let Some(method) = message.get("method").and_then(Value::as_str) {
                if let Some(id) = message.get("id") {
                    let _ = outbox.try_send(answer_to(method, id.clone(), message.get("params")));
                }

                continue;
            }

            if let Some(id) = message.get("id").and_then(Value::as_i64) {
                if let Some(sender) = pending.lock().await.remove(&id) {
                    let _ = sender.send(message);
                }
            }
        }
    });
}
