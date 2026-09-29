use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rumqttc::{
    valid_topic, AsyncClient, ConnectionError, Event, EventLoop, MqttOptions, Outgoing, Packet,
    Publish, QoS, TlsConfiguration, Transport,
};
use serde::Serialize;
use tokio::sync::oneshot;

use crate::engines::db::{tls_config, QueryResult, SessionConfig, TableInfo};
use crate::engines::slicing::{Filter, Op, Slice};

const TOPIC_CAP: usize = 2000;
const MSG_CAP: usize = 200;
const BYTES_CAP: usize = 64 * 1024 * 1024;
const PACKET_CAP: usize = 8 * 1024 * 1024;
const PUBLISH_HEADER: usize = 9;

const RETRY: Duration = Duration::from_secs(1);
const FAREWELL: Duration = Duration::from_millis(500);
const NOTIFY_EVERY: Duration = Duration::from_millis(500);

const COLUMNS: [&str; 4] = ["payload", "qos", "retained", "received"];

type Notify = Box<dyn Fn(&str) + Send>;
type Row = Vec<Option<String>>;

struct Msg {
    payload: String,
    qos: u8,
    retained: bool,
    received: i64,
}

impl Msg {
    fn row(&self) -> Row {
        vec![
            Some(self.payload.clone()),
            Some(self.qos.to_string()),
            Some(self.retained.to_string()),
            Some(self.received.to_string()),
        ]
    }
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
    notify: Option<Notify>,
    dirty: HashSet<String>,
    bytes: usize,
}

impl Shared {
    fn drop_topic(&mut self, topic: &str) {
        if let Some(msgs) = self.topics.remove(topic) {
            self.bytes -= msgs.iter().map(|msg| msg.payload.len()).sum::<usize>();
        }
    }

    fn shed_oldest(&mut self) -> bool {
        let oldest = self
            .topics
            .iter()
            .filter_map(|(name, msgs)| msgs.front().map(|msg| (msg.received, name)))
            .min_by_key(|(received, _)| *received)
            .map(|(_, name)| name.clone());

        let Some(msg) = oldest.and_then(|name| self.topics.get_mut(&name)?.pop_front()) else {
            return false;
        };

        self.bytes -= msg.payload.len();

        true
    }
}

fn qos_of(value: u8) -> Result<QoS, String> {
    match value {
        0 => Ok(QoS::AtMostOnce),
        1 => Ok(QoS::AtLeastOnce),
        2 => Ok(QoS::ExactlyOnce),
        _ => Err("qos is 0, 1 or 2".to_string()),
    }
}

pub struct Mqtt {
    client: AsyncClient,
    shared: Arc<Mutex<Shared>>,
    _stop: oneshot::Sender<()>,
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
                // a login refused over TLS must not be retried with the password in the clear
                Err((error, plaintext_may_follow)) => {
                    failure = error;

                    if !plaintext_may_follow {
                        break;
                    }
                }
            }
        }

        Err(failure)
    }

    async fn connect(
        config: &SessionConfig,
        host: &str,
        port: u16,
        tls: Option<bool>,
    ) -> Result<Self, (String, bool)> {
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
        options.set_max_packet_size(PACKET_CAP, PACKET_CAP);

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
            .map_err(|error| (error.to_string(), false))?;

        // the first poll performs the whole handshake, so a dead or refusing
        // broker is named here instead of surfacing as an empty topic list
        eventloop
            .poll()
            .await
            .map_err(|error| (error.to_string(), matches!(error, ConnectionError::Tls(_))))?;

        let shared = Arc::new(Mutex::new(Shared {
            filters: vec![Subscription { filter, qos: 1 }],
            ..Default::default()
        }));
        let (stop, stopped) = oneshot::channel();

        tokio::spawn(pump(eventloop, client.clone(), shared.clone(), stopped));
        tokio::spawn(announce(Arc::downgrade(&shared)));

        Ok(Mqtt {
            client,
            shared,
            _stop: stop,
        })
    }

    pub fn on_catalog_change(&self, notify: Notify) {
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

        out
    }

    // a topic is a stream, so rows read newest first unless a column is sorted
    pub fn page(&self, table: &str, slice: &Slice) -> Result<QueryResult, String> {
        let rows: Vec<Row> = self
            .shared
            .lock()
            .unwrap()
            .topics
            .get(table)
            .map(|msgs| msgs.iter().rev().map(Msg::row).collect())
            .unwrap_or_default();

        Ok(QueryResult {
            columns: COLUMNS.iter().map(|name| name.to_string()).collect(),
            rows: window(rows, &COLUMNS, slice)?,
            affected: None,
        })
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

        QueryResult {
            columns: Vec::new(),
            rows,
            affected: None,
        }
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

        Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected: Some(1),
        })
    }

    // publish only queues; a packet the event loop refuses later vanishes silently
    pub async fn publish(
        &self,
        topic: &str,
        payload: &str,
        qos: u8,
        retain: bool,
    ) -> Result<(), String> {
        let qos = qos_of(qos)?;

        if topic.is_empty() || !valid_topic(topic) {
            return Err(format!(
                "{topic:?} is not a topic to publish to; + and # only work in filters"
            ));
        }

        if topic.len() + payload.len() + PUBLISH_HEADER > PACKET_CAP {
            return Err(format!(
                "the payload is {} bytes, over the {PACKET_CAP} byte packet limit",
                payload.len()
            ));
        }

        self.client
            .publish(topic, qos, retain, payload.to_string())
            .await
            .map_err(|error| error.to_string())
    }

    // clearing only drops the local buffer; the broker is not asked anything
    pub fn clear(&self, topic: &str) {
        let mut shared = self.shared.lock().unwrap();

        shared.drop_topic(topic);
        shared.order.retain(|name| name != topic);
        shared.dirty.remove(topic);

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

        Ok(())
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

        Ok(())
    }

    pub fn subscriptions(&self) -> Vec<Subscription> {
        self.shared.lock().unwrap().filters.clone()
    }
}

// the event loop holds its own request sender, so RequestsDone never fires
async fn pump(
    mut eventloop: EventLoop,
    client: AsyncClient,
    shared: Arc<Mutex<Shared>>,
    mut stop: oneshot::Receiver<()>,
) {
    loop {
        let polled = tokio::select! {
            _ = &mut stop => break,
            polled = eventloop.poll() => polled,
        };

        match polled {
            Ok(Event::Incoming(Packet::Publish(publish))) => remember(&shared, publish),
            // a reconnect drops every subscription, so they all have to
            // be asked for again or the feed silently stops
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                let filters = shared.lock().unwrap().filters.clone();

                for entry in filters {
                    let _ = client
                        .try_subscribe(entry.filter, qos_of(entry.qos).unwrap_or(QoS::AtLeastOnce));
                }
            }
            Ok(_) => {}
            Err(_) => {
                tokio::select! {
                    _ = &mut stop => break,
                    _ = tokio::time::sleep(RETRY) => {}
                }
            }
        }
    }

    if eventloop.network.is_some() && client.try_disconnect().is_ok() {
        let farewell = async {
            while let Ok(event) = eventloop.poll().await {
                if event == Event::Outgoing(Outgoing::Disconnect) {
                    break;
                }
            }
        };

        let _ = tokio::time::timeout(FAREWELL, farewell).await;
    }
}

// the frontend debounces catalog events, so a steady trickle never lets it reload
async fn announce(shared: Weak<Mutex<Shared>>) {
    let mut tick = tokio::time::interval(NOTIFY_EVERY);

    loop {
        tick.tick().await;

        let Some(shared) = shared.upgrade() else {
            break;
        };

        let mut shared = shared.lock().unwrap();
        let dirty = std::mem::take(&mut shared.dirty);

        if let Some(notify) = &shared.notify {
            for topic in &dirty {
                notify(topic);
            }
        }
    }
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

    let mut guard = shared.lock().unwrap();
    let shared = &mut *guard;

    if !shared.topics.contains_key(&publish.topic) {
        shared.order.push_back(publish.topic.clone());

        while shared.order.len() > TOPIC_CAP {
            let Some(oldest) = shared.order.pop_front() else {
                break;
            };
            shared.drop_topic(&oldest);
        }
    }

    shared.bytes += msg.payload.len();

    let queue = shared.topics.entry(publish.topic.clone()).or_default();

    queue.push_back(msg);

    if queue.len() > MSG_CAP {
        if let Some(old) = queue.pop_front() {
            shared.bytes -= old.payload.len();
        }
    }

    while shared.bytes > BYTES_CAP && shared.shed_oldest() {}

    shared.dirty.insert(publish.topic);
}

pub(crate) fn window(
    mut rows: Vec<Row>,
    columns: &[&str],
    slice: &Slice,
) -> Result<Vec<Row>, String> {
    let at = |name: &str| {
        columns
            .iter()
            .position(|column| *column == name)
            .ok_or_else(|| format!("there is no column named {name}"))
    };

    for filter in &slice.filters {
        let index = at(&filter.column)?;

        rows.retain(|row| keeps(row[index].as_deref(), filter));
    }

    if let Some(sort) = &slice.sort {
        let index = at(&sort.column)?;

        rows.sort_by(|a, b| {
            let order = rank(a[index].as_deref(), b[index].as_deref());

            if sort.descending {
                order.reverse()
            } else {
                order
            }
        });
    }

    let limit = if slice.limit == 0 {
        usize::MAX
    } else {
        slice.limit as usize
    };

    Ok(rows
        .into_iter()
        .skip(slice.offset as usize)
        .take(limit)
        .collect())
}

// mirrors DataGrid's local filter so a filter reads the same pushed down or not
fn keeps(cell: Option<&str>, filter: &Filter) -> bool {
    let Some(cell) = cell else {
        return matches!(filter.op, Op::IsNull);
    };

    let left = cell.to_lowercase();
    let right = filter.value.to_lowercase();

    let order = match (cell.parse::<f64>(), filter.value.parse::<f64>()) {
        (Ok(a), Ok(b)) => a.partial_cmp(&b),
        _ => Some(left.cmp(&right)),
    };

    match filter.op {
        Op::IsNull => false,
        Op::NotNull => true,
        Op::Contains => left.contains(&right),
        Op::Starts => left.starts_with(&right),
        Op::Ends => left.ends_with(&right),
        Op::Eq => order == Some(Ordering::Equal),
        Op::Ne => order != Some(Ordering::Equal),
        Op::Gt => order == Some(Ordering::Greater),
        Op::Gte => matches!(order, Some(Ordering::Greater | Ordering::Equal)),
        Op::Lt => order == Some(Ordering::Less),
        Op::Lte => matches!(order, Some(Ordering::Less | Ordering::Equal)),
    }
}

// sort_by may panic on an inconsistent order, so numbers and text rank apart
fn rank(a: Option<&str>, b: Option<&str>) -> Ordering {
    let number = |cell: Option<&str>| cell.and_then(|text| text.parse::<f64>().ok());

    match (number(a), number(b)) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(&b),
    }
}

#[cfg(test)]
#[path = "mqtt_tests.rs"]
mod mqtt_tests;
