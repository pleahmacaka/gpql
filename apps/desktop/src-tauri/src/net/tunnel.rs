use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use russh::client::{self, Handle};
use russh::keys::key::PublicKey;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Clone, Deserialize, Serialize, Default)]
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
    #[serde(default)]
    pub local_port: String,
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

fn taken(port: u16, error: &std::io::Error) -> String {
    if port == 0 {
        return error.to_string();
    }

    return format!("port {port} on this machine is already in use: {error}");
}

pub fn free(port: u16) -> bool {
    return std::net::TcpListener::bind(("127.0.0.1", port)).is_ok();
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
    let wanted: u16 = config.local_port.trim().parse().unwrap_or(0);
    let listener = TcpListener::bind(("127.0.0.1", wanted))
        .await
        .map_err(|error| taken(wanted, &error))?;
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

#[cfg(test)]
mod live {
    use super::*;

    // GPQL_TEST_SSH=host|port|user|keyPath|passphrase|dbPort
    fn asked() -> Option<(TunnelConfig, u16)> {
        let target = std::env::var("GPQL_TEST_SSH").ok()?;
        let mut parts = target.splitn(6, '|');

        let config = TunnelConfig {
            host: parts.next()?.into(),
            port: parts.next()?.into(),
            user: parts.next()?.into(),
            password: String::new(),
            key_path: parts.next()?.into(),
            passphrase: parts.next()?.into(),
            local_port: String::new(),
        };

        return Some((config, parts.next()?.parse().ok()?));
    }

    #[tokio::test]
    async fn a_driver_reaches_the_database_through_the_jump_host() {
        let Some((config, db_port)) = asked() else {
            return;
        };

        let tunnels = Tunnels::default();
        let hop = open(&tunnels, &config, "127.0.0.1", db_port)
            .await
            .expect("the tunnel did not open");

        let reached = crate::engines::db::SessionConfig {
            kind: "postgres".into(),
            host: "127.0.0.1".into(),
            port: hop.local_port.to_string(),
            user: "postgres".into(),
            database: "postgres".into(),
            tls: "disable".into(),
            read_only: true,
            ..Default::default()
        };

        let session = crate::engines::db::open(&reached)
            .await
            .expect("postgres refused the tunnelled socket");
        let counted =
            crate::engines::db::query(&session, "select count(*) from hop_probe")
                .await
                .expect("the query did not come back");

        assert_eq!(counted.rows[0][0].as_deref(), Some("2"));
    }

    #[tokio::test]
    async fn a_port_already_in_use_is_named_as_such() {
        let Some((mut config, db_port)) = asked() else {
            return;
        };

        let squatter = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let busy = squatter.local_addr().unwrap().port();

        assert!(!free(busy));

        config.local_port = busy.to_string();

        let answer = open(&Tunnels::default(), &config, "127.0.0.1", db_port).await;
        let failure = match answer {
            Ok(_) => panic!("binding a taken port should not succeed"),
            Err(failure) => failure,
        };

        assert!(failure.contains("already in use"), "{failure}");
    }
}

#[cfg(test)]
mod live_url {
    use super::*;

    // GPQL_TEST_SSH_URL=host|port|user|keyPath|passphrase|influxUrl|org|token|bucket
    fn asked() -> Option<(TunnelConfig, crate::engines::db::SessionConfig)> {
        let target = std::env::var("GPQL_TEST_SSH_URL").ok()?;
        let mut parts = target.splitn(9, '|');

        let hop = TunnelConfig {
            host: parts.next()?.into(),
            port: parts.next()?.into(),
            user: parts.next()?.into(),
            password: String::new(),
            key_path: parts.next()?.into(),
            passphrase: parts.next()?.into(),
            local_port: String::new(),
        };

        let config = crate::engines::db::SessionConfig {
            kind: "influxdb2".into(),
            url: parts.next()?.into(),
            user: parts.next()?.into(),
            token: parts.next()?.into(),
            database: parts.next()?.into(),
            read_only: true,
            tunnel: hop.clone(),
            ..Default::default()
        };

        return Some((hop, config));
    }

    #[tokio::test]
    async fn a_url_backend_reaches_its_server_through_the_jump_host() {
        let Some((hop, config)) = asked() else {
            return;
        };

        let address = url::Url::parse(config.url.trim()).unwrap();
        let host = address.host_str().unwrap().to_string();
        let port = address.port_or_known_default().unwrap();

        let tunnels = Tunnels::default();
        let carried = open(&tunnels, &hop, &host, port)
            .await
            .expect("the tunnel did not open");

        let mut reached = config.clone();
        let mut rewritten = address.clone();

        rewritten.set_host(Some("127.0.0.1")).unwrap();
        rewritten.set_port(Some(carried.local_port)).unwrap();
        reached.url = rewritten.to_string();

        assert_ne!(reached.url, config.url);

        let session = crate::engines::db::open(&reached)
            .await
            .expect("influx refused the tunnelled url");
        let listed = crate::engines::introspect::tables(&session)
            .await
            .expect("the bucket did not answer");

        assert!(
            listed.iter().any(|table| table.name == "sensor"),
            "{:?}",
            listed.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
    }
}
