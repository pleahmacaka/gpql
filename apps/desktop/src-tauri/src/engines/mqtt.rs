use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rumqttc::{
    AsyncClient, ConnectionError, Event, EventLoop, MqttOptions, Packet, Publish, QoS,
    TlsConfiguration, Transport,
};
use serde::Serialize;

use crate::engines::db::{tls_config, QueryResult, SessionConfig, TableInfo};
use crate::engines::slicing::Slice;

const TOPIC_CAP: usize = 2000;
const MSG_CAP: usize = 200;

const COLUMNS: [&str; 4] = ["payload", "qos", "retained", "received"];

struct Msg {
    payload: String,
    qos: u8,
    retained: bool,
    received: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub filter: String,
    pub qos: u8,
}

#[derive(Default)]
struct Shared {
    topics: HashMap<String, VecDeque<Msg>>,
    order: VecDeque<String>,
    filters: Vec<Subscription>,
    notify: Option<Box<dyn Fn(&str) + Send>>,
}

fn qos_of(value: u8) -> Result<QoS, String> {
    return match value {
        0 => Ok(QoS::AtMostOnce),
        1 => Ok(QoS::AtLeastOnce),
        2 => Ok(QoS::ExactlyOnce),
        _ => Err("qos is 0, 1 or 2".to_string()),
    };
}

pub struct Mqtt {
    client: AsyncClient,
    shared: Arc<Mutex<Shared>>,
}

impl Mqtt {
    pub async fn open(config: &SessionConfig) -> Result<Self, String> {
        let host = if config.host.is_empty() {
            "127.0.0.1"
        } else {
            config.host.as_str()
        };

        let port: u16 = if config.port.is_empty() {
            1883
        } else {
            config
                .port
                .parse()
                .map_err(|_| "the port is not a number".to_string())?
        };

        let attempts: Vec<Option<bool>> = match config.tls.as_str() {
            "verify-full" => vec![Some(true)],
            "prefer" => vec![Some(false), None],
            "" | "disable" => vec![None],
            _ => vec![Some(false)],
        };

        let mut failure = String::from("could not reach the broker");

        for tls in attempts {
            match Self::connect(config, host, port, tls).await {
                Ok(mqtt) => return Ok(mqtt),
                Err(error) => failure = error,
            }
        }

        return Err(failure);
    }

    async fn connect(
        config: &SessionConfig,
        host: &str,
        port: u16,
        tls: Option<bool>,
    ) -> Result<Self, String> {
        let id = format!(
            "gpql-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let mut options = MqttOptions::new(id, host, port);
        options.set_keep_alive(Duration::from_secs(5));

        if !config.user.is_empty() {
            options.set_credentials(config.user.clone(), config.password.clone());
        }

        if let Some(verify) = tls {
            options.set_transport(Transport::tls_with_config(TlsConfiguration::Rustls(
                Arc::new(tls_config(verify)),
            )));
        }

        let filter = if config.database.is_empty() {
            "#".to_string()
        } else {
            config.database.clone()
        };

        let (client, mut eventloop) = AsyncClient::new(options, 100);

        client
            .subscribe(filter.clone(), QoS::AtLeastOnce)
            .await
            .map_err(|error| error.to_string())?;

        // the first poll performs the whole handshake, so a dead or refusing
        // broker is named here instead of surfacing as an empty topic list
        handshake(&mut eventloop).await?;

        let shared = Arc::new(Mutex::new(Shared {
            filters: vec![Subscription { filter, qos: 1 }],
            ..Default::default()
        }));
        let into = shared.clone();
        let again = client.clone();

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(publish))) => remember(&into, publish),
                    // a reconnect drops every subscription, so they all have to
                    // be asked for again or the feed silently stops
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        let filters = into.lock().unwrap().filters.clone();

                        for entry in filters {
                            let _ = again.try_subscribe(
                                entry.filter,
                                qos_of(entry.qos).unwrap_or(QoS::AtLeastOnce),
                            );
                        }
                    }
                    Ok(_) => {}
                    Err(ConnectionError::RequestsDone) => break,
                    Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                }
            }
        });

        return Ok(Mqtt { client, shared });
    }

    pub fn on_catalog_change(&self, notify: Box<dyn Fn(&str) + Send>) {
        self.shared.lock().unwrap().notify = Some(notify);
    }

    pub fn tables(&self) -> Vec<TableInfo> {
        let shared = self.shared.lock().unwrap();

        let mut out: Vec<TableInfo> = shared
            .topics
            .iter()
            .map(|(name, msgs)| TableInfo {
                name: name.clone(),
                rows: msgs.len() as i64,
            })
            .collect();

        out.sort_by(|a, b| a.name.cmp(&b.name));

        return out;
    }

    pub fn page(&self, table: &str, slice: &Slice) -> Result<QueryResult, String> {
        let shared = self.shared.lock().unwrap();
        let mut picked: Vec<&Msg> = Vec::new();

        if let Some(msgs) = shared.topics.get(table) {
            // a topic is a stream, so the buffer reads newest first unless the
            // user picked a direction on a column
            let descending = slice
                .sort
                .as_ref()
                .map(|sort| sort.descending)
                .unwrap_or(true);

            let ordered: Box<dyn Iterator<Item = &Msg>> = if descending {
                Box::new(msgs.iter().rev())
            } else {
                Box::new(msgs.iter())
            };

            let window = ordered.skip(slice.offset as usize);

            picked = if slice.limit == 0 {
                window.collect()
            } else {
                window.take(slice.limit as usize).collect()
            };
        }

        let rows = picked
            .into_iter()
            .map(|msg| {
                vec![
                    Some(msg.payload.clone()),
                    Some(msg.qos.to_string()),
                    Some(msg.retained.to_string()),
                    Some(msg.received.to_string()),
                ]
            })
            .collect();

        return Ok(QueryResult {
            columns: COLUMNS.iter().map(|name| name.to_string()).collect(),
            rows,
            affected: None,
        });
    }

    pub fn columns(&self) -> QueryResult {
        let shared = self.shared.lock().unwrap();
        let mut topics: Vec<&String> = shared.topics.keys().collect();
        let mut rows = Vec::new();

        topics.sort();

        for topic in topics {
            for name in COLUMNS {
                rows.push(vec![
                    Some(topic.clone()),
                    Some(name.to_string()),
                    Some("text".to_string()),
                    Some(String::new()),
                    Some(String::new()),
                    Some(String::new()),
                    Some(String::new()),
                ]);
            }
        }

        return QueryResult {
            columns: Vec::new(),
            rows,
            affected: None,
        };
    }

    pub async fn query(&self, sql: &str) -> Result<QueryResult, String> {
        let rest = sql.trim_start();
        let (verb, rest) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));

        if !verb.eq_ignore_ascii_case("publish") {
            return Err("mqtt has no query language; browse topics in the data tab".to_string());
        }

        let rest = rest.trim_start();
        let (topic, payload) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));

        if topic.is_empty() {
            return Err("publish needs a topic and a payload".to_string());
        }

        self.publish(topic, payload.trim_start(), 1, false).await?;

        return Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected: Some(1),
        });
    }

    pub async fn publish(
        &self,
        topic: &str,
        payload: &str,
        qos: u8,
        retain: bool,
    ) -> Result<(), String> {
        self.client
            .publish(topic, qos_of(qos)?, retain, payload.to_string())
            .await
            .map_err(|error| error.to_string())
    }

    // clearing only drops the local buffer; the broker is not asked anything
    pub fn clear(&self, topic: &str) {
        let mut shared = self.shared.lock().unwrap();

        shared.topics.remove(topic);
        shared.order.retain(|name| name != topic);

        if let Some(notify) = &shared.notify {
            notify(topic);
        }
    }

    pub async fn subscribe(&self, filter: &str, qos: u8) -> Result<(), String> {
        let qos = qos_of(qos)?;

        self.client
            .subscribe(filter.to_string(), qos)
            .await
            .map_err(|error| error.to_string())?;

        let mut shared = self.shared.lock().unwrap();
        let qos = qos as u8;

        match shared
            .filters
            .iter_mut()
            .find(|entry| entry.filter == filter)
        {
            Some(held) => held.qos = qos,
            None => shared.filters.push(Subscription {
                filter: filter.to_string(),
                qos,
            }),
        }

        return Ok(());
    }

    pub async fn unsubscribe(&self, filter: &str) -> Result<(), String> {
        self.client
            .unsubscribe(filter.to_string())
            .await
            .map_err(|error| error.to_string())?;

        self.shared
            .lock()
            .unwrap()
            .filters
            .retain(|entry| entry.filter != filter);

        return Ok(());
    }

    pub fn subscriptions(&self) -> Vec<Subscription> {
        return self.shared.lock().unwrap().filters.clone();
    }
}

async fn handshake(eventloop: &mut EventLoop) -> Result<(), String> {
    return eventloop
        .poll()
        .await
        .map(|_| ())
        .map_err(|error| error.to_string());
}

fn remember(shared: &Mutex<Shared>, publish: Publish) {
    let payload = match std::str::from_utf8(&publish.payload) {
        Ok(text) => text.to_string(),
        Err(_) => format!("{} bytes", publish.payload.len()),
    };

    let msg = Msg {
        payload,
        qos: publish.qos as u8,
        retained: publish.retain,
        received: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64,
    };

    let mut shared = shared.lock().unwrap();

    let fresh = !shared.topics.contains_key(&publish.topic);

    if fresh {
        shared.order.push_back(publish.topic.clone());

        while shared.order.len() > TOPIC_CAP {
            let Some(oldest) = shared.order.pop_front() else {
                break;
            };
            shared.topics.remove(&oldest);
        }
    }

    let queue = shared.topics.entry(publish.topic.clone()).or_default();

    if queue.len() == MSG_CAP {
        queue.pop_front();
    }

    queue.push_back(msg);

    if let Some(notify) = &shared.notify {
        notify(&publish.topic);
    }
}

#[cfg(test)]
#[path = "mqtt_tests.rs"]
mod mqtt_tests;
