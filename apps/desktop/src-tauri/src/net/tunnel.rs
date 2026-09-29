use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::key::PublicKey;
use russh::Preferred;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use tokio::net::TcpListener;
use tokio::time::timeout;

const DIAL_WAIT: Duration = Duration::from_secs(10);
const AUTH_WAIT: Duration = Duration::from_secs(15);

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
        !self.host.trim().is_empty()
    }
}

struct Guard {
    host: String,
    port: u16,
}

#[derive(Debug)]
enum Refusal {
    Ssh(russh::Error),
    Key(String),
}

impl From<russh::Error> for Refusal {
    fn from(error: russh::Error) -> Self {
        Refusal::Ssh(error)
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Ssh(error) => write!(out, "ssh: {error}"),
            Refusal::Key(message) => out.write_str(message),
        }
    }
}

#[async_trait::async_trait]
impl client::Handler for Guard {
    type Error = Refusal;

    async fn check_server_key(&mut self, key: &PublicKey) -> Result<bool, Self::Error> {
        let (host, port, key) = (self.host.clone(), self.port, key.clone());

        tokio::task::spawn_blocking(move || trust(&host, port, &key))
            .await
            .map_err(|error| Refusal::Key(error.to_string()))?
            .map_err(Refusal::Key)
    }
}

fn learned_hosts() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|folder| folder.join("gpql").join("known_hosts"))
        .ok_or_else(|| "no app data folder on this machine".to_string())
}

fn changed(host: &str, port: u16, key: &PublicKey, file: &Path, line: usize) -> String {
    format!(
        "the host key of {host}:{port} does not match the one on line {line} of {}; the server now offers SHA256:{}. If the jump host was rebuilt, remove that line and connect again",
        file.display(),
        key.fingerprint()
    )
}

struct Record {
    file: PathBuf,
    line: usize,
    revoked: bool,
    key: PublicKey,
}

fn hashed_name(wanted: &str, hashed: &str) -> bool {
    use base64::Engine;
    use hmac::Mac;

    let decode = |part| base64::engine::general_purpose::STANDARD.decode(part);

    let Some((salt, hash)) = hashed.split_once('|') else {
        return false;
    };

    let (Ok(salt), Ok(hash)) = (decode(salt), decode(hash)) else {
        return false;
    };

    hmac::Hmac::<sha1::Sha1>::new_from_slice(&salt)
        .is_ok_and(|mac| mac.chain_update(wanted).verify_slice(&hash).is_ok())
}

fn names(wanted: &str, hosts: &str) -> bool {
    hosts
        .split(',')
        .any(|entry| match entry.strip_prefix("|1|") {
            Some(hashed) => hashed_name(wanted, hashed),
            None => entry == wanted,
        })
}

// a key type russh can't read skips its line instead of failing the whole file
fn records(host: &str, port: u16, file: &Path) -> Vec<Record> {
    let Ok(text) = std::fs::read_to_string(file) else {
        return Vec::new();
    };

    let wanted = if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    };

    text.lines()
        .enumerate()
        .filter_map(|(at, line)| {
            let mut fields = line.split_whitespace();
            let mut hosts = fields.next()?;
            let revoked = hosts == "@revoked";

            if revoked {
                hosts = fields.next()?;
            }

            if hosts.starts_with(['#', '@']) || !names(&wanted, hosts) {
                return None;
            }

            let _kind = fields.next()?;
            let key = russh::keys::parse_public_key_base64(fields.next()?).ok()?;

            Some(Record {
                file: file.to_path_buf(),
                line: at + 1,
                revoked,
                key,
            })
        })
        .collect()
}

fn on_record(host: &str, port: u16, learned: &Path) -> Vec<Record> {
    let theirs = dirs::home_dir().map(|home| home.join(".ssh").join("known_hosts"));

    theirs
        .iter()
        .map(PathBuf::as_path)
        .chain([learned])
        .flat_map(|file| records(host, port, file))
        .collect()
}

fn family(name: &'static str) -> &'static str {
    if name.starts_with("rsa-sha2") {
        "ssh-rsa"
    } else {
        name
    }
}

// asking only for key types already on record is how openssh avoids a false alarm
fn host_key_order(host: &str, port: u16) -> Preferred {
    let Ok(learned) = learned_hosts() else {
        return Preferred::default();
    };

    let recorded: Vec<&str> = on_record(host, port, &learned)
        .iter()
        .filter(|record| !record.revoked)
        .map(|record| family(record.key.name()))
        .collect();

    let usable: Vec<russh::keys::key::Name> = Preferred::DEFAULT
        .key
        .iter()
        .filter(|name| recorded.contains(&family(name.0)))
        .copied()
        .collect();

    if usable.is_empty() {
        return Preferred::default();
    }

    Preferred {
        key: Cow::Owned(usable),
        ..Preferred::default()
    }
}

// an unknown jump host is learned into gpql's own file, never into ~/.ssh/known_hosts
fn trust(host: &str, port: u16, key: &PublicKey) -> Result<bool, String> {
    let learned = learned_hosts()?;
    let known = on_record(host, port, &learned);

    if let Some(record) = known
        .iter()
        .find(|record| record.revoked && record.key == *key)
    {
        return Err(format!(
            "the host key of {host}:{port} is marked revoked on line {} of {}",
            record.line,
            record.file.display()
        ));
    }

    if known
        .iter()
        .any(|record| !record.revoked && record.key == *key)
    {
        return Ok(true);
    }

    let kept: Vec<&Record> = known.iter().filter(|record| !record.revoked).collect();

    if let Some(record) = kept
        .iter()
        .find(|record| family(record.key.name()) == family(key.name()))
    {
        return Err(changed(host, port, key, &record.file, record.line));
    }

    // russh only compares keys of the same type, so another type used to pass unseen
    if let Some(record) = kept.first() {
        return Err(format!(
            "{host}:{port} offers a {} host key (SHA256:{}), while line {} of {} records only a {} key for it; add the offered key to that file if you trust it",
            key.name(),
            key.fingerprint(),
            record.line,
            record.file.display(),
            record.key.name()
        ));
    }

    russh::keys::learn_known_hosts_path(host, port, key, &learned).map_err(|error| {
        format!(
            "the host key of {host}:{port} could not be recorded in {}: {error}",
            learned.display()
        )
    })?;

    Ok(true)
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
        self.next.fetch_add(1, Ordering::Relaxed)
    }
}

async fn connect(config: &TunnelConfig) -> Result<Handle<Guard>, String> {
    let host = config.host.trim().to_string();
    let port: u16 = config.port.trim().parse().unwrap_or(22);
    let (asked, dialled) = (host.clone(), port);
    let preferred = tokio::task::spawn_blocking(move || host_key_order(&asked, dialled))
        .await
        .unwrap_or_default();
    let settings = Arc::new(client::Config {
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        inactivity_timeout: Some(Duration::from_secs(90)),
        preferred,
        ..Default::default()
    });
    let guard = Guard {
        host: host.clone(),
        port,
    };

    let mut session = timeout(
        DIAL_WAIT,
        client::connect(settings, (host.as_str(), port), guard),
    )
    .await
    .map_err(|_| {
        format!(
            "ssh: {host}:{port} did not answer within {} seconds",
            DIAL_WAIT.as_secs()
        )
    })?
    .map_err(|refusal| refusal.to_string())?;

    let authed = timeout(AUTH_WAIT, authenticate(&mut session, config))
        .await
        .map_err(|_| {
            format!(
                "ssh: {host} did not finish signing in within {} seconds",
                AUTH_WAIT.as_secs()
            )
        })??;

    if !authed {
        return Err("ssh: the server refused those credentials".into());
    }

    Ok(session)
}

async fn authenticate(session: &mut Handle<Guard>, config: &TunnelConfig) -> Result<bool, String> {
    let user = config.user.trim();

    if config.key_path.trim().is_empty() {
        return session
            .authenticate_password(user, &config.password)
            .await
            .map_err(|error| format!("ssh: {error}"));
    }

    let path = config.key_path.trim().to_string();
    let pass = (!config.passphrase.is_empty()).then(|| config.passphrase.clone());

    let key =
        tokio::task::spawn_blocking(move || russh::keys::load_secret_key(path, pass.as_deref()))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| format!("ssh key: {error}"))?;

    session
        .authenticate_publickey(user, Arc::new(key))
        .await
        .map_err(|error| format!("ssh: {error}"))
}

fn taken(port: u16, error: &std::io::Error) -> String {
    if port == 0 {
        return error.to_string();
    }

    format!("port {port} on this machine is already in use: {error}")
}

pub fn free(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
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

            // one client resetting its socket (WSAECONNRESET) must not end the tunnel
            let Ok((mut socket, _)) = accepted else {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
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

    Ok(Tunnel { local_port, stop })
}

#[cfg(test)]
mod known_hosts {
    use super::*;

    const FIRST: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIKRMCN5DcxTKcPCEgVjf0bf0cX82mRqQBBw8+VwOHcMS";
    const SECOND: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIJZjMdAczilgiBILn1rz00fHaRU5uSGoDG8+w4yS7qT9";

    #[test]
    fn every_recorded_key_counts_and_an_unreadable_line_is_skipped() {
        let file = std::env::temp_dir().join(format!("gpql-known-{}", std::process::id()));
        let text = format!(
            "# comment\n\
             [jump.example]:2222 sk-ssh-ed25519@openssh.com AAAAnotakey\n\
             |1|hbTB8jLQntyptZ4ENsc8tjbqCFY=|68rcHnV2SnkQFqmFSeFthLr0Tws= ssh-ed25519 {FIRST}\n\
             other.example ssh-ed25519 {SECOND}\n\
             @revoked [jump.example]:2222 ssh-ed25519 {SECOND}\n"
        );

        std::fs::write(&file, text).unwrap();

        let found = records("jump.example", 2222, &file);
        let other_port = records("jump.example", 22, &file);
        let _ = std::fs::remove_file(&file);

        let first = russh::keys::parse_public_key_base64(FIRST).unwrap();
        let second = russh::keys::parse_public_key_base64(SECOND).unwrap();

        assert_eq!(found.len(), 2);
        assert!(found[0].key == first && !found[0].revoked && found[0].line == 3);
        assert!(found[1].key == second && found[1].revoked);
        assert!(other_port.is_empty());
    }
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

        Some((config, parts.next()?.parse().ok()?))
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
        let counted = crate::engines::db::query(&session, "select count(*) from hop_probe")
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

        Some((hop, config))
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
