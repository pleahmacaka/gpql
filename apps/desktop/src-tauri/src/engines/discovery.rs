use std::net::TcpStream;
use std::time::Duration;

use serde::Serialize;

use super::db::{open, query, SessionConfig};

const PG_DATABASES: &str = "select datname from pg_database
     where datistemplate = false and datallowconn = true
     order by datname";

const SHOW_DATABASES: &str = "show databases";

const OPEN_WAIT: Duration = Duration::from_secs(4);
const QUERY_WAIT: Duration = Duration::from_secs(4);

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

fn discovery(
    target: &Target,
    user: &str,
    password: &str,
    database: &str,
    needs_login: bool,
) -> Discovery {
    return Discovery {
        kind: target.kind.clone(),
        host: target.host.clone(),
        port: target.port.to_string(),
        user: user.to_string(),
        password: password.to_string(),
        database: database.to_string(),
        detail: target.detail.clone(),
        needs_login,
    };
}

pub fn local_postgres_ports() -> Vec<u16> {
    return (5432..=5435)
        .filter(|port| reachable("127.0.0.1", *port, 120))
        .collect();
}

pub fn reachable(host: &str, port: u16, patience: u64) -> bool {
    use std::net::ToSocketAddrs;

    let Ok(mut addresses) = (host, port).to_socket_addrs() else {
        return false;
    };

    return addresses.any(|address| {
        TcpStream::connect_timeout(&address, Duration::from_millis(patience)).is_ok()
    });
}

fn wants_login(error: &str) -> bool {
    let text = error.to_lowercase();

    return [
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
    .any(|needle| text.contains(needle));
}

fn probe_config(
    target: &Target,
    user: &str,
    password: &str,
    database: &str,
    read_only: bool,
) -> SessionConfig {
    return SessionConfig {
        kind: target.kind.clone(),
        host: target.host.clone(),
        port: target.port.to_string(),
        user: user.to_string(),
        password: password.to_string(),
        database: database.to_string(),
        path: String::new(),
        read_only,
        tls: "prefer".into(),
        url: String::new(),
        token: String::new(),
        warehouse: String::new(),
        schema: String::new(),
        ..Default::default()
    };
}

async fn probe_wire(
    target: &Target,
    candidates: &[(String, String)],
    database: &str,
    listing: &str,
    read_only: bool,
    default_user: &str,
) -> Vec<Discovery> {
    let mut found = Vec::new();
    let mut tries = candidates.to_vec();

    tries.push((default_user.to_string(), String::new()));
    tries.dedup();

    let mut reached = false;
    let mut auth_seen = false;

    for (user, password) in &tries {
        let probe = probe_config(target, user, password, database, read_only);

        let session = match tokio::time::timeout(OPEN_WAIT, open(&probe)).await {
            Ok(Ok(session)) => session,
            Ok(Err(error)) => {
                auth_seen |= wants_login(&error);
                continue;
            }
            Err(_) => continue,
        };

        let Ok(Ok(result)) = tokio::time::timeout(QUERY_WAIT, query(&session, listing)).await
        else {
            continue;
        };

        reached = true;

        for row in result.rows {
            found.push(discovery(
                target,
                user,
                password,
                &row[0].clone().unwrap_or_default(),
                false,
            ));
        }

        break;
    }

    if !reached && (target.kind == "postgres" || auth_seen) {
        let user = candidates
            .first()
            .map(|(user, _)| user.clone())
            .unwrap_or_else(|| default_user.to_string());

        found.push(discovery(target, &user, "", "", true));
    }

    return found;
}

async fn probe_clickhouse(target: &Target, candidates: &[(String, String)]) -> Vec<Discovery> {
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

    let mut tries = vec![("default".to_string(), String::new())];
    tries.extend(candidates.iter().cloned());
    tries.dedup();

    for (user, password) in tries {
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

    return vec![discovery(target, "default", "", "", true)];
}

async fn probe_mqtt(target: &Target) -> Vec<Discovery> {
    let probe = probe_config(target, "", "", "#", false);

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

    return match ping {
        Ok(_) => vec![discovery(target, "", "", "falkordb", false)],
        Err(error) if wants_login(&error.to_string()) => {
            vec![discovery(target, "", "", "", true)]
        }
        Err(_) => Vec::new(),
    };
}

async fn probe(target: &Target, candidates: &[(String, String)]) -> Vec<Discovery> {
    return match target.kind.as_str() {
        "postgres" => {
            probe_wire(
                target,
                candidates,
                "postgres",
                PG_DATABASES,
                true,
                "postgres",
            )
            .await
        }
        "mysql" => {
            probe_wire(
                target,
                candidates,
                "information_schema",
                SHOW_DATABASES,
                false,
                "root",
            )
            .await
        }
        "greptimedb" => {
            probe_wire(
                target,
                candidates,
                "public",
                SHOW_DATABASES,
                false,
                "greptime",
            )
            .await
        }
        "clickhouse" => probe_clickhouse(target, candidates).await,
        "mqtt" => probe_mqtt(target).await,
        "falkordb" => probe_falkordb(target).await,
        "influxdb2" | "influxdb" => vec![discovery(target, "", "", "", true)],
        "neo4j" => vec![discovery(target, "neo4j", "", "", true)],
        _ => Vec::new(),
    };
}

pub async fn scan_host(
    host: &str,
    ports: &[u16],
    candidates: &[(String, String)],
) -> Vec<Discovery> {
    let mut found = Vec::new();

    for port in ports.iter().copied() {
        let target = Target {
            kind: "postgres".into(),
            host: host.to_string(),
            port,
            detail: String::new(),
        };

        if reachable(host, port, 200) {
            found.extend(
                probe_wire(
                    &target,
                    candidates,
                    "postgres",
                    PG_DATABASES,
                    true,
                    "postgres",
                )
                .await,
            );
        }
    }

    return found;
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

    return Some(match name.as_str() {
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
        _ => return None,
    });
}

fn serves(kind: &str, container_port: u16) -> bool {
    return match kind {
        "postgres" => container_port == 5432,
        "mysql" => container_port == 3306,
        "falkordb" => container_port == 6379,
        "clickhouse" => container_port == 8123,
        "influxdb" => container_port == 8181,
        "influxdb2" => container_port == 8086,
        "neo4j" => container_port == 7687,
        "greptimedb" => container_port == 4003,
        "mqtt" => matches!(container_port, 1883 | 8883 | 8884),
        _ => false,
    };
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

    return out;
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

    return targets;
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

    return String::from_utf8(output.stdout).ok();
}

pub async fn scan(candidates: &[(String, String)]) -> Vec<Discovery> {
    let mut targets = tokio::task::spawn_blocking(|| {
        return docker_ps()
            .map(|output| parse_ps(&output))
            .unwrap_or_default();
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

    let mut found = Vec::new();

    for target in &targets {
        if !reachable(&target.host, target.port, 300) {
            continue;
        }

        found.extend(probe(target, candidates).await);
    }

    return found;
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
        if std::process::Command::new("nanomq")
            .arg("--help")
            .output()
            .is_err()
        {
            return;
        }
        if crate::engines::backends::find("mqtt").is_none() {
            return;
        }
        if reachable("127.0.0.1", 1883, 100) {
            return;
        }

        let mut broker = Broker(
            std::process::Command::new("nanomq")
                .args(["start", "--port", "1883"])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("nanomq should start"),
        );

        let mut up = false;

        for _ in 0..50 {
            if reachable("127.0.0.1", 1883, 100) {
                up = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        let found = if up { scan(&[]).await } else { Vec::new() };

        broker.0.kill().ok();
        drop(broker);

        assert!(up, "nanomq never opened its port");
        assert!(
            found
                .iter()
                .any(|entry| entry.kind == "mqtt" && entry.port == "1883"),
            "{found:?}"
        );
    }
}
