use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use russh::client::{self, Handle};
use russh::keys::key::PublicKey;
use serde::Deserialize;
use tokio::net::TcpListener;

#[derive(Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TunnelConfig {
    pub host: String,
    #[serde(default)]
    pub port: String,
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub key_path: String,
    #[serde(default)]
    pub passphrase: String,
}

impl TunnelConfig {
    pub fn wanted(&self) -> bool {
        return !self.host.trim().is_empty();
    }
}

struct Blind;

#[async_trait::async_trait]
impl client::Handler for Blind {
    type Error = russh::Error;

    // a jump host is named by the user, and gpql keeps no known_hosts file, so
    // there is nothing here to check a key against
    async fn check_server_key(&mut self, _key: &PublicKey) -> Result<bool, Self::Error> {
        return Ok(true);
    }
}

pub struct Tunnel {
    pub local_port: u16,
    stop: tokio::sync::watch::Sender<bool>,
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}

#[derive(Default)]
pub struct Tunnels {
    open: Mutex<HashMap<String, Arc<Tunnel>>>,
    next: AtomicU16,
}

impl Tunnels {
    pub fn keep(&self, id: &str, tunnel: Tunnel) {
        self.open
            .lock()
            .unwrap()
            .insert(id.to_string(), Arc::new(tunnel));
    }

    pub fn drop_for(&self, id: &str) {
        self.open.lock().unwrap().remove(id);
    }

    pub fn clear(&self) {
        self.open.lock().unwrap().clear();
    }

    fn ticket(&self) -> u16 {
        return self.next.fetch_add(1, Ordering::Relaxed);
    }
}

async fn connect(config: &TunnelConfig) -> Result<Handle<Blind>, String> {
    let port: u16 = config.port.trim().parse().unwrap_or(22);
    let settings = Arc::new(client::Config::default());

    let mut session =
        client::connect(settings, (config.host.trim(), port), Blind)
            .await
            .map_err(|error| format!("ssh: {error}"))?;

    let authed = if config.key_path.trim().is_empty() {
        session
            .authenticate_password(config.user.trim(), &config.password)
            .await
            .map_err(|error| format!("ssh: {error}"))?
    } else {
        let pass = (!config.passphrase.is_empty()).then_some(config.passphrase.as_str());
        let key = russh::keys::load_secret_key(config.key_path.trim(), pass)
            .map_err(|error| format!("ssh key: {error}"))?;

        session
            .authenticate_publickey(config.user.trim(), Arc::new(key))
            .await
            .map_err(|error| format!("ssh: {error}"))?
    };

    if !authed {
        return Err("ssh: the server refused those credentials".into());
    }

    return Ok(session);
}

// binds a loopback port and forwards it over ssh, so every driver keeps
// talking plain tcp and knows nothing about the jump host
pub async fn open(
    tunnels: &Tunnels,
    config: &TunnelConfig,
    target_host: &str,
    target_port: u16,
) -> Result<Tunnel, String> {
    let session = Arc::new(connect(config).await?);
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|error| error.to_string())?;
    let local_port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();

    let _ = tunnels.ticket();

    let (stop, mut halt) = tokio::sync::watch::channel(false);
    let host = target_host.to_string();

    tokio::spawn(async move {
        loop {
            let accepted = tokio::select! {
                _ = halt.changed() => return,
                accepted = listener.accept() => accepted,
            };

            let Ok((mut socket, _)) = accepted else {
                return;
            };

            let session = session.clone();
            let host = host.clone();

            tokio::spawn(async move {
                let Ok(channel) = session
                    .channel_open_direct_tcpip(host, target_port as u32, "127.0.0.1", 0)
                    .await
                else {
                    return;
                };

                let mut stream = channel.into_stream();

                // tokio already knows how to shuttle two sockets both ways
                let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
            });
        }
    });

    return Ok(Tunnel { local_port, stop });
}
