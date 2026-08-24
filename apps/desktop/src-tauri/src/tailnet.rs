use std::process::Command;

use serde::Serialize;
use serde_json::Value;

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

pub fn peers() -> Vec<Peer> {
    let Some(raw) = status() else {
        return Vec::new();
    };
    let Ok(status): Result<Value, _> = serde_json::from_str(&raw) else {
        return Vec::new();
    };

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

    return out;
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
        online: peer
            .get("Online")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    });
}

fn status() -> Option<String> {
    for candidate in CANDIDATES {
        let mut command = Command::new(candidate);
        command.args(["status", "--json"]);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }

        if let Ok(output) = command.output() {
            if output.status.success() {
                return String::from_utf8(output.stdout).ok();
            }
        }
    }

    return None;
}
