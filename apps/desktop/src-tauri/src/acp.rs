use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, oneshot};
use tokio::time::{Duration, timeout};

const PATIENCE: Duration = Duration::from_secs(120);

struct Agent {
    child: Child,
    stdin: ChildStdin,
    next: AtomicI64,
    session: Option<String>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
    spoken: Arc<Mutex<String>>,
}

#[derive(Default)]
pub struct Assistant {
    agent: Mutex<Option<Agent>>,
}

impl Assistant {
    pub async fn start(&self, program: &str, args: &[String]) -> Result<(), String> {
        self.stop().await;

        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        #[cfg(windows)]
        command.creation_flags(0x0800_0000);

        let mut child = command.spawn().map_err(|error| error.to_string())?;
        let stdin = child.stdin.take().ok_or("the agent took no input")?;
        let stdout = child.stdout.take().ok_or("the agent gave no output")?;

        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let spoken = Arc::new(Mutex::new(String::new()));

        listen(stdout, pending.clone(), spoken.clone());

        let mut agent = Agent {
            child,
            stdin,
            next: AtomicI64::new(1),
            session: None,
            pending,
            spoken,
        };

        agent
            .request(
                "initialize",
                json!({
                    "protocolVersion": 1,
                    "clientCapabilities": {
                        "fs": { "readTextFile": false, "writeTextFile": false }
                    },
                }),
            )
            .await?;

        let session = agent
            .request(
                "session/new",
                json!({ "cwd": ".", "mcpServers": [] }),
            )
            .await?;

        agent.session = session
            .get("sessionId")
            .and_then(Value::as_str)
            .map(str::to_string);

        *self.agent.lock().await = Some(agent);

        return Ok(());
    }

    pub async fn stop(&self) {
        if let Some(mut agent) = self.agent.lock().await.take() {
            let _ = agent.child.kill().await;
        }
    }

    pub async fn ready(&self) -> bool {
        return self.agent.lock().await.is_some();
    }

    pub async fn ask(&self, prompt: &str) -> Result<String, String> {
        let mut holder = self.agent.lock().await;
        let Some(agent) = holder.as_mut() else {
            return Err("no agent is running".into());
        };

        let session = agent
            .session
            .clone()
            .ok_or_else(|| "the agent never opened a session".to_string())?;

        agent.spoken.lock().await.clear();
        agent
            .request(
                "session/prompt",
                json!({
                    "sessionId": session,
                    "prompt": [{ "type": "text", "text": prompt }],
                }),
            )
            .await?;

        let said = agent.spoken.lock().await.clone();

        return Ok(said.trim().to_string());
    }
}

impl Agent {
    async fn send(&mut self, payload: Value) -> Result<(), String> {
        let line = format!("{payload}\n");

        return self
            .stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|error| error.to_string());
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
            .map_err(|_| "the agent went away".to_string())?;

        if let Some(message) = answer.pointer("/error/message").and_then(Value::as_str) {
            return Err(message.to_string());
        }

        return Ok(answer.get("result").cloned().unwrap_or(Value::Null));
    }
}

fn listen(
    stdout: tokio::process::ChildStdout,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
    spoken: Arc<Mutex<String>>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(message): Result<Value, _> = serde_json::from_str(&line) else {
                continue;
            };

            if let Some(id) = message.get("id").and_then(Value::as_i64) {
                if message.get("method").is_some() {
                    continue;
                }

                if let Some(sender) = pending.lock().await.remove(&id) {
                    let _ = sender.send(message);
                }

                continue;
            }

            if message.get("method").and_then(Value::as_str) == Some("session/update") {
                let chunk = message
                    .pointer("/params/update/content/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                let kind = message
                    .pointer("/params/update/sessionUpdate")
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                if kind == "agent_message_chunk" && !chunk.is_empty() {
                    spoken.lock().await.push_str(chunk);
                }
            }
        }
    });
}
