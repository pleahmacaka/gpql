use std::time::Duration;

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const WINDOW: Duration = Duration::from_secs(180);

const DONE: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><meta charset=utf-8><title>gpql</title><body style=\"font:14px system-ui;padding:3rem\">Done. You can close this tab.</body>";

const REJECTED: &str =
    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nnot this window";

pub async fn sign_in(site: &str) -> Result<String, String> {
    let listener = bind().await?;
    let port = port_of(&listener)?;
    let nonce = nonce();

    open(&format!("{site}/account?port={port}&state={nonce}"))?;

    return catch(listener, "token", Some(nonce)).await;
}

pub async fn openrouter() -> Result<String, String> {
    let listener = bind().await?;
    let port = port_of(&listener)?;
    let verifier = nonce() + &nonce();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));

    open(&format!(
        "https://openrouter.ai/auth?callback_url=http%3A%2F%2F127.0.0.1%3A{port}&code_challenge={challenge}&code_challenge_method=S256"
    ))?;

    let code = catch(listener, "code", None).await?;

    let answer: serde_json::Value = reqwest::Client::new()
        .post("https://openrouter.ai/api/v1/auth/keys")
        .json(&serde_json::json!({
            "code": code,
            "code_verifier": verifier,
            "code_challenge_method": "S256",
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    return answer
        .get("key")
        .and_then(|key| key.as_str())
        .map(str::to_string)
        .ok_or_else(|| "openrouter did not hand back a key".to_string());
}

async fn bind() -> Result<TcpListener, String> {
    return TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| error.to_string());
}

fn port_of(listener: &TcpListener) -> Result<u16, String> {
    return listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| error.to_string());
}

fn open(url: &str) -> Result<(), String> {
    return tauri_plugin_opener::open_url(url, None::<&str>)
        .map_err(|error| error.to_string());
}

async fn catch(
    listener: TcpListener,
    key: &str,
    nonce: Option<String>,
) -> Result<String, String> {
    return tokio::time::timeout(WINDOW, collect(listener, key, nonce))
        .await
        .map_err(|_| "the browser window closed before it finished".to_string())?;
}

async fn collect(
    listener: TcpListener,
    key: &str,
    nonce: Option<String>,
) -> Result<String, String> {
    loop {
        let Ok((mut stream, _)) = listener.accept().await else {
            continue;
        };

        match read_value(&mut stream, key, nonce.as_deref()).await {
            Some(value) => {
                let _ = stream.write_all(DONE.as_bytes()).await;
                let _ = stream.shutdown().await;

                return Ok(value);
            }
            None => {
                let _ = stream.write_all(REJECTED.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        }
    }
}

async fn read_value(
    stream: &mut TcpStream,
    key: &str,
    nonce: Option<&str>,
) -> Option<String> {
    let mut buffer = [0u8; 8192];
    let read = stream.read(&mut buffer).await.ok()?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request.split_whitespace().nth(1)?;
    let query = target.split_once('?')?.1;

    let mut wanted = None;
    let mut state = None;

    for pair in query.split('&') {
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };

        if name == key {
            wanted = Some(decode(value));
        }

        if name == "state" {
            state = Some(decode(value));
        }
    }

    if let Some(expected) = nonce {
        if !same(state.as_deref()?, expected) {
            return None;
        }
    }

    return wanted.filter(|value| !value.is_empty());
}

fn same(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }

    return left
        .bytes()
        .zip(right.bytes())
        .fold(0u8, |seen, (a, b)| seen | (a ^ b))
        == 0;
}

fn decode(raw: &str) -> String {
    let bytes = raw.replace('+', " ").into_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");

            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }

        out.push(bytes[index]);
        index += 1;
    }

    return String::from_utf8_lossy(&out).into_owned();
}

fn nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);

    return bytes.iter().map(|byte| format!("{byte:02x}")).collect();
}
