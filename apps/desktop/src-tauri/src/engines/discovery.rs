use std::net::IpAddr;
use std::time::Duration;

use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use super::db::{open, query, SessionConfig};

const PG_DATABASES: &str = "select datname from pg_database
     where datistemplate = false and datallowconn = true
     order by datname";

const SHOW_DATABASES: &str = "show databases";

const OPEN_WAIT: Duration = Duration::from_secs(4);
const QUERY_WAIT: Duration = Duration::from_secs(4);
const PARALLEL: usize = 8;

const LOCAL_PORTS: &[(&str, u16)] = &[
    ("postgres", 5432),
    ("postgres", 5433),
    ("postgres", 5434),
    ("postgres", 5435),
    ("mysql", 3306),
    ("mysql", 3307),
    ("mysql", 3308),
    ("greptimedb", 4003),
    ("mqtt", 1883),
    ("s3", 9000),
    ("s3", 4566),
    ("falkordb", 6379),
    ("clickhouse", 8123),
    ("influxdb2", 8086),
    ("influxdb", 8181),
    ("neo4j", 7687),
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub kind: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub detail: String,
    pub needs_login: bool,
}

struct Target {
    kind: String,
    host: String,
    port: u16,
    detail: String,
}

#[derive(Clone)]
pub struct Key {
    pub user: String,
    pub password: String,
    pub tls: String,
}

pub struct Saved {
    pub host: String,
    pub port: u16,
    pub key: Key,
}

#[derive(Default)]
pub struct Keyring {
    pub saved: Vec<Saved>,
    pub presets: Vec<Key>,
}

impl Key {
    fn blank(user: &str) -> Self {
        Key {
            user: user.to_string(),
            password: String::new(),
            tls: String::new(),
        }
    }
}

fn loopback(host: &str) -> bool {
    let host = host.trim().trim_matches(['[', ']']);

    host.is_empty()
        || host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn same_host(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim()) || (loopback(left) && loopback(right))
}

impl Keyring {
    // a saved secret only goes back to the address it was saved for, a preset only to this machine
    fn keys_for(&self, target: &Target) -> Vec<Key> {
        let mut keys: Vec<Key> = self
            .saved
            .iter()
            .filter(|saved| saved.port == target.port && same_host(&saved.host, &target.host))
            .map(|saved| saved.key.clone())
            .collect();

        if loopback(&target.host) {
            keys.extend(self.presets.iter().cloned());
        }

        keys
    }
}

fn distinct(keys: impl IntoIterator<Item = Key>) -> Vec<Key> {
    let mut out: Vec<Key> = Vec::new();

    for key in keys {
        if !out
            .iter()
            .any(|seen| seen.user == key.user && seen.password == key.password)
        {
            out.push(key);
        }
    }

    out
}

// these probes speak plain http, where a password travels as readable text
fn over_plain_http(target: &Target, keys: Vec<Key>) -> Vec<Key> {
    if loopback(&target.host) {
        return keys;
    }

    keys.into_iter()
        .filter(|key| key.password.is_empty())
        .collect()
}

fn discovery(
    target: &Target,
    user: &str,
    password: &str,
    database: &str,
    needs_login: bool,
) -> Discovery {
    Discovery {
        kind: target.kind.clone(),
        host: target.host.clone(),
        port: target.port.to_string(),
        user: user.to_string(),
        password: password.to_string(),
        database: database.to_string(),
        detail: target.detail.clone(),
        needs_login,
    }
}

pub async fn local_postgres_ports() -> Vec<u16> {
    let mut open = Vec::new();

    for port in 5432..=5435 {
        if reachable("127.0.0.1", port, 120).await {
            open.push(port);
        }
    }

    open
}

pub async fn reachable(host: &str, port: u16, patience: u64) -> bool {
    let patience = Duration::from_millis(patience);

    let Ok(Ok(addresses)) = timeout(patience, tokio::net::lookup_host((host, port))).await else {
        return false;
    };

    for address in addresses {
        if let Ok(Ok(_)) = timeout(patience, TcpStream::connect(address)).await {
            return true;
        }
    }

    false
}

fn wants_login(error: &str) -> bool {
    let text = error.to_lowercase();

    [
        "auth",
        "password",
        "credential",
        "401",
        "403",
        "unauthor",
        "permission",
        "denied",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn probe_config(target: &Target, key: &Key, database: &str, read_only: bool) -> SessionConfig {
    SessionConfig {
        kind: target.kind.clone(),
        host: target.host.clone(),
        port: target.port.to_string(),
        user: key.user.clone(),
        password: key.password.clone(),
        database: database.to_string(),
        read_only,
        tls: if key.tls.is_empty() {
            "prefer".into()
        } else {
            key.tls.clone()
        },
        ..Default::default()
    }
}

async fn probe_wire(
    target: &Target,
    keys: &[Key],
    database: &str,
    listing: &str,
    read_only: bool,
    default_user: &str,
) -> Vec<Discovery> {
    let mut found = Vec::new();
    let tries = distinct(keys.iter().cloned().chain([Key::blank(default_user)]));

    let mut reached = false;
    let mut auth_seen = false;

    for key in &tries {
        let probe = probe_config(target, key, database, read_only);

        let session = match timeout(OPEN_WAIT, open(&probe)).await {
            Ok(Ok(session)) => session,
            Ok(Err(error)) => {
                auth_seen |= wants_login(&error);
                continue;
            }
            Err(_) => continue,
        };

        let Ok(Ok(result)) = timeout(QUERY_WAIT, query(&session, listing)).await else {
            continue;
        };

        reached = true;

        for row in result.rows {
            found.push(discovery(
                target,
                &key.user,
                &key.password,
                &row[0].clone().unwrap_or_default(),
                false,
            ));
        }

        break;
    }

    if !reached && (target.kind == "postgres" || auth_seen) {
        let user = keys
            .first()
            .map(|key| key.user.clone())
            .unwrap_or_else(|| default_user.to_string());

        found.push(discovery(target, &user, "", "", true));
    }

    found
}

async fn probe_clickhouse(target: &Target, keys: &[Key]) -> Vec<Discovery> {
    let base = format!("http://{}:{}", target.host, target.port);

    let Ok(client) = reqwest::Client::builder().timeout(QUERY_WAIT).build() else {
        return Vec::new();
    };

    let Ok(ping) = client.get(format!("{base}/ping")).send().await else {
        return Vec::new();
    };

    if !ping.status().is_success() {
        return Vec::new();
    }

    let tries = over_plain_http(
        target,
        distinct(
            [Key::blank("default")]
                .into_iter()
                .chain(keys.iter().cloned()),
        ),
    );

    for Key { user, password, .. } in tries {
        let answer = client
            .post(&base)
            .header("X-ClickHouse-User", &user)
            .header("X-ClickHouse-Key", &password)
            .query(&[("query", SHOW_DATABASES)])
            .send()
            .await;

        let Ok(response) = answer else {
            continue;
        };

        if response.status().is_success() {
            let body = response.text().await.unwrap_or_default();

            return body
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|name| discovery(target, &user, &password, name, false))
                .collect();
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if !wants_login(&format!("{status} {body}")) {
            return Vec::new();
        }
    }

    vec![discovery(target, "default", "", "", true)]
}

async fn probe_mqtt(target: &Target) -> Vec<Discovery> {
    let probe = probe_config(target, &Key::blank(""), "#", false);

    return match tokio::time::timeout(OPEN_WAIT, open(&probe)).await {
        Ok(Ok(_)) => vec![discovery(target, "", "", "#", false)],
        Ok(Err(error)) if wants_login(&error) => {
            vec![discovery(target, "", "", "", true)]
        }
        _ => Vec::new(),
    };
}

async fn probe_falkordb(target: &Target) -> Vec<Discovery> {
    let url = format!("redis://{}:{}", target.host, target.port);

    let Ok(client) = redis::Client::open(url) else {
        return Vec::new();
    };

    let mut connection =
        match tokio::time::timeout(OPEN_WAIT, client.get_multiplexed_async_connection()).await {
            Ok(Ok(connection)) => connection,
            Ok(Err(error)) => {
                return if wants_login(&error.to_string()) {
                    vec![discovery(target, "", "", "", true)]
                } else {
                    Vec::new()
                };
            }
            Err(_) => return Vec::new(),
        };

    let ping: Result<String, redis::RedisError> =
        redis::cmd("PING").query_async(&mut connection).await;

    match ping {
        Ok(_) => vec![discovery(target, "", "", "falkordb", false)],
        Err(error) if wants_login(&error.to_string()) => {
            vec![discovery(target, "", "", "", true)]
        }
        Err(_) => Vec::new(),
    }
}

// an open port alone proves nothing, so the endpoint has to answer like s3:
// its own error codes, a server banner, or localstack's health json
async fn probe_s3(target: &Target, keys: &[Key]) -> Vec<Discovery> {
    let base = format!("http://{}:{}", target.host, target.port);

    let Ok(client) = reqwest::Client::builder().timeout(QUERY_WAIT).build() else {
        return Vec::new();
    };

    let Ok(response) = client.get(&base).send().await else {
        return Vec::new();
    };

    let banner = response
        .headers()
        .get("server")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_lowercase();
    let body = response.text().await.unwrap_or_default();

    let mut looks_s3 = banner.contains("minio")
        || body.contains("<ListAllMyBucketsResult")
        || (body.contains("<Error")
            && [
                "AccessDenied",
                "SignatureDoesNotMatch",
                "InvalidAccessKeyId",
                "AccessDeniedException",
                "AllAccessDisabled",
            ]
            .iter()
            .any(|code| body.contains(code)));

    if !looks_s3 {
        if let Ok(health) = client
            .get(format!("{base}/_localstack/health"))
            .send()
            .await
        {
            if health.status().is_success() {
                looks_s3 = health.text().await.unwrap_or_default().contains("\"s3\"");
            }
        }
    }

    if !looks_s3 {
        return Vec::new();
    }

    let minio = Key {
        password: "minioadmin".into(),
        ..Key::blank("minioadmin")
    };
    let tries = over_plain_http(
        target,
        distinct(keys.iter().cloned().chain([minio, Key::blank("")])),
    );

    for key in tries {
        let mut probe = probe_config(target, &key, "", false);
        probe.url = base.clone();

        let Ok(Ok(session)) = timeout(OPEN_WAIT, open(&probe)).await else {
            continue;
        };

        if let Ok(Ok(_)) = timeout(QUERY_WAIT, query(&session, "ls")).await {
            return vec![discovery(target, &key.user, &key.password, "", false)];
        }
    }

    vec![discovery(target, "", "", "", true)]
}

async fn probe(target: &Target, keys: &[Key]) -> Vec<Discovery> {
    match target.kind.as_str() {
        "postgres" => probe_wire(target, keys, "postgres", PG_DATABASES, true, "postgres").await,
        "mysql" => {
            probe_wire(
                target,
                keys,
                "information_schema",
                SHOW_DATABASES,
                false,
                "root",
            )
            .await
        }
        "greptimedb" => probe_wire(target, keys, "public", SHOW_DATABASES, false, "greptime").await,
        "clickhouse" => probe_clickhouse(target, keys).await,
        "mqtt" => probe_mqtt(target).await,
        "s3" => probe_s3(target, keys).await,
        "falkordb" => probe_falkordb(target).await,
        "influxdb2" | "influxdb" => vec![discovery(target, "", "", "", true)],
        "neo4j" => vec![discovery(target, "neo4j", "", "", true)],
        _ => Vec::new(),
    }
}

async fn probe_all(targets: &[Target], keyring: &Keyring, patience: u64) -> Vec<Discovery> {
    let gate = Semaphore::new(PARALLEL);

    let probed = futures_util::future::join_all(targets.iter().map(|target| async {
        let Ok(_turn) = gate.acquire().await else {
            return Vec::new();
        };

        if !reachable(&target.host, target.port, patience).await {
            return Vec::new();
        }

        probe(target, &keyring.keys_for(target)).await
    }))
    .await;

    probed.into_iter().flatten().collect()
}

pub async fn scan_hosts(hosts: &[String], ports: &[u16], keyring: &Keyring) -> Vec<Discovery> {
    let targets: Vec<Target> = hosts
        .iter()
        .flat_map(|host| {
            ports.iter().map(|port| Target {
                kind: "postgres".into(),
                host: host.clone(),
                port: *port,
                detail: String::new(),
            })
        })
        .collect();

    probe_all(&targets, keyring, 200).await
}

fn image_kind(image: &str) -> Option<&'static str> {
    let last = image.rsplit('/').next().unwrap_or(image);
    let name = last
        .split(['@', ':'])
        .next()
        .unwrap_or_default()
        .to_lowercase();

    if name.is_empty() || name.starts_with("mongo") {
        return None;
    }

    Some(match name.as_str() {
        _ if name.starts_with("postgres")
            || name.starts_with("postgis")
            || name.starts_with("timescale") =>
        {
            "postgres"
        }
        _ if name.starts_with("mysql") || name.starts_with("mariadb") => "mysql",
        _ if name.starts_with("redis") || name.starts_with("falkordb") => "falkordb",
        _ if name.starts_with("clickhouse") => "clickhouse",
        _ if name.starts_with("influxdb3") => "influxdb",
        _ if name.starts_with("influxdb") => "influxdb2",
        _ if name.starts_with("neo4j") => "neo4j",
        _ if name.starts_with("greptime") => "greptimedb",
        _ if [
            "nanomq",
            "emqx",
            "eclipse-mosquitto",
            "mosquitto",
            "hivemq",
            "vernemq",
        ]
        .iter()
        .any(|prefix| name.starts_with(prefix)) =>
        {
            "mqtt"
        }
        _ if ["minio", "localstack", "rustfs", "seaweedfs", "garage"]
            .iter()
            .any(|prefix| name.starts_with(prefix)) =>
        {
            "s3"
        }
        _ => return None,
    })
}

fn serves(kind: &str, container_port: u16) -> bool {
    match kind {
        "postgres" => container_port == 5432,
        "mysql" => container_port == 3306,
        "falkordb" => container_port == 6379,
        "clickhouse" => container_port == 8123,
        "influxdb" => container_port == 8181,
        "influxdb2" => container_port == 8086,
        "neo4j" => container_port == 7687,
        "greptimedb" => container_port == 4003,
        "mqtt" => matches!(container_port, 1883 | 8883 | 8884),
        "s3" => matches!(container_port, 9000 | 8333 | 3900 | 4566 | 7480),
        _ => false,
    }
}

fn published(field: &str) -> Vec<(String, u16, u16)> {
    let mut out = Vec::new();

    for mapping in field.split(',') {
        let Some((left, right)) = mapping.trim().split_once("->") else {
            continue;
        };
        let Some((advertised, host_port)) = left.rsplit_once(':') else {
            continue;
        };
        let Some(port) = host_port
            .split('-')
            .next()
            .and_then(|part| part.parse::<u16>().ok())
        else {
            continue;
        };
        let Some(container_port) = right
            .split(['/', '-'])
            .next()
            .and_then(|part| part.parse::<u16>().ok())
        else {
            continue;
        };

        let host = match advertised.trim_matches(['[', ']']) {
            "" | "0.0.0.0" | "::" => "127.0.0.1".to_string(),
            other => other.to_string(),
        };

        out.push((host, port, container_port));
    }

    out
}

fn parse_ps(output: &str) -> Vec<Target> {
    let mut targets = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in output.lines() {
        let fields: Vec<&str> = line.split('\t').collect();

        if fields.len() < 4 {
            continue;
        }

        let Some(kind) = image_kind(fields[1]) else {
            continue;
        };

        for (host, port, container_port) in published(fields[3]) {
            if !serves(kind, container_port) || !seen.insert((kind, port)) {
                continue;
            }

            targets.push(Target {
                kind: kind.to_string(),
                host,
                port,
                detail: fields[2].to_string(),
            });
        }
    }

    targets
}

fn docker_ps() -> Option<String> {
    let mut child = std::process::Command::new("docker")
        .args([
            "ps",
            "--format",
            "{{.ID}}\t{{.Image}}\t{{.Names}}\t{{.Ports}}",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let deadline = std::time::Instant::now() + Duration::from_secs(3);

    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();

                return None;
            }
            Err(_) => return None,
        }
    }

    let output = child.wait_with_output().ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()
}

pub async fn scan(keyring: &Keyring) -> Vec<Discovery> {
    let mut targets = tokio::task::spawn_blocking(|| {
        docker_ps()
            .map(|output| parse_ps(&output))
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    for (kind, port) in LOCAL_PORTS {
        if targets
            .iter()
            .any(|target| target.kind == *kind && target.port == *port)
        {
            continue;
        }

        targets.push(Target {
            kind: kind.to_string(),
            host: "127.0.0.1".to_string(),
            port: *port,
            detail: String::new(),
        });
    }

    probe_all(&targets, keyring, 300).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_image_maps_to_a_kind() {
        for (image, kind) in [
            ("postgres:16", "postgres"),
            ("docker.io/library/postgres:16", "postgres"),
            ("postgis/postgis", "postgres"),
            ("timescale/timescaledb:latest-pg16", "postgres"),
            ("mysql:8", "mysql"),
            ("mysql/mysql-server", "mysql"),
            ("mariadb:11", "mysql"),
            ("redis:7", "falkordb"),
            ("redis/redis-stack", "falkordb"),
            ("falkordb/falkordb", "falkordb"),
            ("clickhouse/clickhouse-server", "clickhouse"),
            ("influxdb:2.7", "influxdb2"),
            ("influxdb3-core", "influxdb"),
            ("neo4j:5", "neo4j"),
            ("greptime/greptimedb", "greptimedb"),
            ("nanomq", "mqtt"),
            ("emqx", "mqtt"),
            ("emqx/emqx-enterprise", "mqtt"),
            ("eclipse-mosquitto", "mqtt"),
            ("mosquitto", "mqtt"),
            ("hivemq/hivemq-ce", "mqtt"),
            ("vernemq/vernemq", "mqtt"),
        ] {
            assert_eq!(image_kind(image), Some(kind), "{image}");
        }
    }

    #[test]
    fn unsupported_images_are_skipped() {
        for image in [
            "mongo:7",
            "mongodb/mongodb-community-server",
            "ubuntu",
            "nginx",
        ] {
            assert_eq!(image_kind(image), None, "{image}");
        }
    }

    #[test]
    fn only_published_ports_become_targets() {
        let found = published("0.0.0.0:5432->5432/tcp, 5433/tcp");

        assert_eq!(found, vec![("127.0.0.1".to_string(), 5432, 5432)]);
    }

    #[test]
    fn every_published_prefix_form_is_read() {
        assert_eq!(
            published("127.0.0.1:3307->3306/tcp"),
            vec![("127.0.0.1".to_string(), 3307, 3306)]
        );
        assert_eq!(
            published(":::5433->5432/tcp"),
            vec![("127.0.0.1".to_string(), 5433, 5432)]
        );
        assert_eq!(
            published("[::1]:6379->6379/tcp"),
            vec![("::1".to_string(), 6379, 6379)]
        );
        assert_eq!(
            published("192.168.1.5:8123->8123/tcp"),
            vec![("192.168.1.5".to_string(), 8123, 8123)]
        );
    }

    #[test]
    fn docker_lines_yield_one_target_per_service_port() {
        let output = concat!(
            "a1\tpostgres:16\tpg-main\t0.0.0.0:5432->5432/tcp, :::5432->5432/tcp\n",
            "b2\tmysql:8\tmy-db\t127.0.0.1:3307->3306/tcp\n",
            "c3\tmongo:7\tmongo\t0.0.0.0:27017->27017/tcp\n",
            "d4\tredis:7\tcache\t6379/tcp\n",
            "e5\temqx:5\tbroker\t0.0.0.0:1883->1883/tcp, 0.0.0.0:8083->8083/tcp, 0.0.0.0:18083->18083/tcp\n",
            "f6\tneo4j:5\tgraph\t0.0.0.0:7474->7474/tcp, 0.0.0.0:7687->7687/tcp\n",
            "g7\tclickhouse:25\tkitchen\t0.0.0.0:18123->8123/tcp, 0.0.0.0:9000->9000/tcp\n",
        );

        let targets = parse_ps(output);
        let spots: Vec<(&str, u16, &str, &str)> = targets
            .iter()
            .map(|target| {
                (
                    target.kind.as_str(),
                    target.port,
                    target.detail.as_str(),
                    target.host.as_str(),
                )
            })
            .collect();

        assert_eq!(
            spots,
            vec![
                ("postgres", 5432, "pg-main", "127.0.0.1"),
                ("mysql", 3307, "my-db", "127.0.0.1"),
                ("mqtt", 1883, "broker", "127.0.0.1"),
                ("neo4j", 7687, "graph", "127.0.0.1"),
                ("clickhouse", 18123, "kitchen", "127.0.0.1"),
            ]
        );
    }

    struct Broker(std::process::Child);

    impl Drop for Broker {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[tokio::test]
    async fn a_running_mqtt_broker_is_found() {
        if std::process::Command::new("mosquitto")
            .arg("-h")
            .output()
            .is_err()
        {
            return;
        }
        if crate::engines::backends::find("mqtt").is_none() {
            return;
        }
        if reachable("127.0.0.1", 1883, 100).await {
            return;
        }

        let mut broker = Broker(
            std::process::Command::new("mosquitto")
                .args(["-p", "1883"])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("mosquitto should start"),
        );

        let mut up = false;

        for _ in 0..50 {
            if reachable("127.0.0.1", 1883, 100).await {
                up = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let found = if up {
            scan(&Keyring::default()).await
        } else {
            Vec::new()
        };

        broker.0.kill().ok();
        drop(broker);

        assert!(up, "mosquitto never opened its port");
        assert!(
            found
                .iter()
                .any(|entry| entry.kind == "mqtt" && entry.port == "1883"),
            "{found:?}"
        );
    }
}
