use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

const WINDOW: Duration = Duration::from_secs(180);
const READ_WAIT: Duration = Duration::from_secs(5);
const LARGEST: usize = 64 * 1024;

const DONE: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>GPQL</title><body style=\"margin:0;min-height:100vh;display:grid;place-items:center;font:15px system-ui,sans-serif;background:#141414;color:#ececec\"><main style=\"text-align:center\"><h1 style=\"font-size:20px;font-weight:600;margin:0 0 8px\">GPQL is connected</h1><p style=\"margin:0;opacity:.7\">You can close this tab and go back to GPQL.</p></main></body></html>";

const REJECTED: &str =
    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nnot this window";

struct Expect {
    key: &'static str,
    state: Option<String>,
    path: Option<String>,
}

struct Request {
    method: String,
    target: String,
    body: Vec<u8>,
}

pub async fn sign_in(site: &str) -> Result<String, String> {
    let listener = bind().await?;
    let port = port_of(&listener)?;
    let nonce = nonce();

    open(&format!("{site}/account?port={port}&state={nonce}&form=1"))?;

    catch(
        listener,
        Expect {
            key: "token",
            state: Some(nonce),
            path: None,
        },
    )
    .await
}

pub async fn openrouter() -> Result<String, String> {
    let listener = bind().await?;
    let port = port_of(&listener)?;
    let verifier = nonce() + &nonce();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));

    // openrouter echoes no state, so an unguessable callback path stands in for it
    let path = format!("/{}", nonce());
    let callback = format!("http://127.0.0.1:{port}{path}");
    let start = url::Url::parse_with_params(
        "https://openrouter.ai/auth",
        [
            ("callback_url", callback.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ],
    )
    .map_err(|error| error.to_string())?;

    open(start.as_str())?;

    let code = catch(
        listener,
        Expect {
            key: "code",
            state: None,
            path: Some(path),
        },
    )
    .await?;

    let response = reqwest::Client::new()
        .post("https://openrouter.ai/api/v1/auth/keys")
        .json(&serde_json::json!({
            "code": code,
            "code_verifier": verifier,
            "code_challenge_method": "S256",
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?;

    super::answer(response)
        .await?
        .get("key")
        .and_then(|key| key.as_str())
        .map(str::to_string)
        .ok_or_else(|| "openrouter did not hand back a key".to_string())
}

async fn bind() -> Result<TcpListener, String> {
    return TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| error.to_string());
}

fn port_of(listener: &TcpListener) -> Result<u16, String> {
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| error.to_string())
}

fn open(url: &str) -> Result<(), String> {
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

async fn catch(listener: TcpListener, expect: Expect) -> Result<String, String> {
    timeout(WINDOW, collect(listener, Arc::new(expect)))
        .await
        .map_err(|_| "the browser window closed before it finished".to_string())
}

// one task per connection, so a silent local socket cannot stall the real callback
async fn collect(listener: TcpListener, expect: Arc<Expect>) -> String {
    let (found, mut arrived) = tokio::sync::mpsc::channel::<String>(1);

    loop {
        tokio::select! {
            Some(value) = arrived.recv() => return value,
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                };

                let (found, expect) = (found.clone(), expect.clone());

                tokio::spawn(async move {
                    if let Some(value) = reply(stream, &expect).await {
                        let _ = found.send(value).await;
                    }
                });
            }
        }
    }
}

async fn reply(mut stream: TcpStream, expect: &Expect) -> Option<String> {
    let value = timeout(READ_WAIT, read_request(&mut stream))
        .await
        .ok()
        .flatten()
        .and_then(|request| expect.pick(&request));

    let page = if value.is_some() { DONE } else { REJECTED };

    let _ = timeout(READ_WAIT, stream.write_all(page.as_bytes())).await;
    let _ = stream.shutdown().await;

    value
}

async fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];

    let head_end = loop {
        let read = stream.read(&mut chunk).await.ok()?;

        if read == 0 || buffer.len() + read > LARGEST {
            return None;
        }

        buffer.extend_from_slice(&chunk[..read]);

        if let Some(at) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break at + 4;
        }
    };

    let head = String::from_utf8_lossy(&buffer[..head_end]).into_owned();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let target = first.next()?.to_string();

    let length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);

    if head_end + length > LARGEST {
        return None;
    }

    while buffer.len() < head_end + length {
        let read = stream.read(&mut chunk).await.ok()?;

        if read == 0 {
            return None;
        }

        buffer.extend_from_slice(&chunk[..read]);
    }

    Some(Request {
        method,
        target,
        body: buffer[head_end..head_end + length].to_vec(),
    })
}

impl Expect {
    fn pick(&self, request: &Request) -> Option<String> {
        let (path, query) = request
            .target
            .split_once('?')
            .unwrap_or((request.target.as_str(), ""));

        if self.path.as_deref().is_some_and(|wanted| wanted != path) {
            return None;
        }

        let fields = match request.method.as_str() {
            "GET" => query.as_bytes(),
            "POST" => request.body.as_slice(),
            _ => return None,
        };

        let mut value = None;
        let mut state = None;

        for (name, found) in url::form_urlencoded::parse(fields) {
            if name == self.key {
                value = Some(found.into_owned());
            } else if name == "state" {
                state = Some(found.into_owned());
            }
        }

        if let Some(expected) = &self.state {
            if !same(state.as_deref()?, expected) {
                return None;
            }
        }

        value.filter(|value| !value.is_empty())
    }
}

fn same(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.bytes()
        .zip(right.bytes())
        .fold(0u8, |seen, (a, b)| seen | (a ^ b))
        == 0
}

fn nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);

    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
