use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub name: String,
    pub host: String,
    pub online: bool,
}

const CANDIDATES: [&str; 3] = [
    "tailscale",
    r"C:\Program Files\Tailscale\tailscale.exe",
    "/usr/bin/tailscale",
];

const PATIENCE: Duration = Duration::from_secs(5);

pub async fn peers() -> Result<Vec<Peer>, String> {
    let raw = status().await?;
    let status: Value =
        serde_json::from_str(&raw).map_err(|error| format!("tailscale status: {error}"))?;

    if let Some(state) = status.get("BackendState").and_then(Value::as_str) {
        if state != "Running" {
            return Err(format!("tailscale is not connected ({state})"));
        }
    }

    let mut out = Vec::new();

    if let Some(me) = status.get("Self") {
        push(&mut out, me);
    }

    if let Some(map) = status.get("Peer").and_then(Value::as_object) {
        for peer in map.values() {
            push(&mut out, peer);
        }
    }

    out.sort_by(|a, b| b.online.cmp(&a.online).then(a.name.cmp(&b.name)));

    Ok(out)
}

fn push(out: &mut Vec<Peer>, peer: &Value) {
    let Some(host) = peer
        .get("TailscaleIPs")
        .and_then(Value::as_array)
        .and_then(|ips| ips.first())
        .and_then(Value::as_str)
    else {
        return;
    };

    let name = peer
        .get("DNSName")
        .and_then(Value::as_str)
        .map(|dns| dns.trim_end_matches('.').split('.').next().unwrap_or(dns))
        .unwrap_or(host)
        .to_string();

    out.push(Peer {
        name,
        host: host.to_string(),
        online: peer.get("Online").and_then(Value::as_bool).unwrap_or(false),
    });
}

async fn status() -> Result<String, String> {
    let mut refusal = None;

    for candidate in CANDIDATES {
        let mut command = Command::new(candidate);

        command
            .args(["status", "--json"])
            .stdin(Stdio::null())
            .kill_on_drop(true);

        #[cfg(windows)]
        command.creation_flags(0x0800_0000);

        let output = match timeout(PATIENCE, command.output()).await {
            Err(_) => {
                return Err(format!(
                    "tailscale did not answer within {} seconds",
                    PATIENCE.as_secs()
                ));
            }
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Ok(Err(error)) => {
                refusal = Some(format!("tailscale: {error}"));
                continue;
            }
            Ok(Ok(output)) => output,
        };

        if output.status.success() {
            return String::from_utf8(output.stdout).map_err(|error| error.to_string());
        }

        let said = String::from_utf8_lossy(&output.stderr).trim().to_string();

        refusal = Some(if said.is_empty() {
            format!("tailscale status failed ({})", output.status)
        } else {
            said
        });
    }

    Err(refusal.unwrap_or_else(|| "tailscale is not installed on this machine".to_string()))
}
