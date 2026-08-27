use std::net::TcpStream;
use std::time::Duration;

use serde::Serialize;

use super::db::{open, query, SessionConfig};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub needs_login: bool,
}

pub fn local_postgres_ports() -> Vec<u16> {
    return (5432..=5435).filter(|port| reachable("127.0.0.1", *port, 120)).collect();
}

pub fn reachable(host: &str, port: u16, patience: u64) -> bool {
    use std::net::ToSocketAddrs;

    let Ok(mut addresses) = (host, port).to_socket_addrs() else {
        return false;
    };

    return addresses
        .any(|address| TcpStream::connect_timeout(&address, Duration::from_millis(patience)).is_ok());
}

pub async fn scan_host(
    host: &str,
    ports: &[u16],
    candidates: &[(String, String)],
) -> Vec<Discovery> {
    let mut found = Vec::new();

    for port in ports.iter().copied().filter(|port| reachable(host, *port, 200)) {
        let mut reached = false;

        for (user, password) in candidates {
            let probe = SessionConfig {
                kind: "postgres".into(),
                host: host.to_string(),
                port: port.to_string(),
                user: user.clone(),
                password: password.clone(),
                database: "postgres".into(),
                path: String::new(),
                warehouse: String::new(),
                schema: String::new(),
                read_only: true,
                tls: "prefer".into(),
                url: String::new(),
                token: String::new(),
                ..Default::default()
            };

            let Ok(session) = open(&probe).await else {
                continue;
            };
            let Ok(result) = query(
                &session,
                "select datname from pg_database
                 where datistemplate = false and datallowconn = true
                 order by datname",
            )
            .await
            else {
                continue;
            };

            reached = true;

            for row in result.rows {
                found.push(Discovery {
                    host: host.to_string(),
                    port: port.to_string(),
                    user: user.clone(),
                    password: password.clone(),
                    database: row[0].clone().unwrap_or_default(),
                    needs_login: false,
                });
            }

            break;
        }

        if !reached {
            found.push(Discovery {
                host: host.to_string(),
                port: port.to_string(),
                user: candidates
                    .first()
                    .map(|(user, _)| user.clone())
                    .unwrap_or_else(|| "postgres".into()),
                password: String::new(),
                database: String::new(),
                needs_login: true,
            });
        }
    }

    return found;
}

pub async fn scan(candidates: &[(String, String)]) -> Vec<Discovery> {
    return scan_host("127.0.0.1", &[5432, 5433, 5434, 5435], candidates).await;
}
