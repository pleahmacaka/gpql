use super::*;

use crate::engines::db::{open, Session};
use crate::engines::objects::objects;
use crate::engines::slicing::table_rows;

// end to end against a real broker; needs nanomq on PATH, so a machine
// without one is not failed
static BROKER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Broker(std::process::Child);

impl Broker {
    async fn start() -> Option<(Broker, u16)> {
        if std::process::Command::new("nanomq")
            .arg("--help")
            .output()
            .is_err()
        {
            eprintln!("nanomq is not installed; skipping the live mqtt test");
            return None;
        }

        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let child = std::process::Command::new("nanomq")
            .args(["start", "--url", &format!("nmq-tcp://127.0.0.1:{port}")])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;

        let broker = Broker(child);

        for _ in 0..50 {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                // the listener can accept before the broker finishes its own
                // setup, so give it a beat before the real connect
                tokio::time::sleep(Duration::from_millis(300)).await;

                return Some((broker, port));
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        panic!("nanomq did not open its port");
    }
}

impl Drop for Broker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn open_test(port: u16, read_only: bool) -> Session {
    let config = SessionConfig {
        kind: "mqtt".into(),
        host: "127.0.0.1".into(),
        port: port.to_string(),
        database: "gpql/#".into(),
        read_only,
        ..Default::default()
    };

    for _ in 0..10 {
        match open(&config).await {
            Ok(session) => return session,
            Err(_) => tokio::time::sleep(Duration::from_millis(300)).await,
        }
    }

    panic!("could not open the test broker");
}

async fn seed(port: u16) {
    let mut options = MqttOptions::new("gpql-test-seed", "127.0.0.1", port);
    options.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(options, 10);

    tokio::spawn(async move { while let Ok(_) = eventloop.poll().await {} });

    for index in 1..=3 {
        client
            .publish(
                "gpql/sensors",
                QoS::AtLeastOnce,
                false,
                format!("reading {index}"),
            )
            .await
            .unwrap();
    }
}

async fn listed(session: &Session, topic: &str, want: i64) -> Vec<TableInfo> {
    for _ in 0..50 {
        let found = crate::engines::introspect::tables(session).await.unwrap();

        if found
            .iter()
            .any(|table| table.name == topic && table.rows >= want)
        {
            return found;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    return crate::engines::introspect::tables(session).await.unwrap();
}

#[tokio::test]
async fn topics_show_up_as_tables_with_their_messages() {
    let _guard = BROKER_LOCK.lock().await;
    let Some((_broker, port)) = Broker::start().await else {
        return;
    };

    let session = open_test(port, true).await;
    seed(port).await;

    let found = listed(&session, "gpql/sensors", 3).await;
    let topic = found
        .iter()
        .find(|table| table.name == "gpql/sensors")
        .expect("the topic never showed up");

    let names: Vec<&str> = found.iter().map(|table| table.name.as_str()).collect();

    assert!(topic.rows >= 3, "{names:?}");

    let page = table_rows(&session, "gpql/sensors", &Slice::default())
        .await
        .unwrap();

    assert_eq!(page.columns, ["payload", "qos", "retained", "received"]);

    let payloads: Vec<&str> = page
        .rows
        .iter()
        .map(|row| row[0].as_deref().unwrap_or_default())
        .collect();

    assert!(payloads.contains(&"reading 1"), "{payloads:?}");
    assert!(payloads.contains(&"reading 2"), "{payloads:?}");
    assert!(payloads.contains(&"reading 3"), "{payloads:?}");

    let objects = objects(&session).await.unwrap();

    assert!(objects.is_empty(), "a broker has no catalog objects");
}

#[tokio::test]
async fn read_only_refuses_to_publish() {
    let _guard = BROKER_LOCK.lock().await;
    let Some((_broker, port)) = Broker::start().await else {
        return;
    };

    let session = open_test(port, true).await;
    let failure = crate::engines::db::query(&session, "publish gpql/sensors hi").await;

    assert_eq!(failure.err().as_deref(), Some("this session is read only"));
}

#[tokio::test]
async fn a_publish_reaches_every_subscriber() {
    let _guard = BROKER_LOCK.lock().await;
    let Some((_broker, port)) = Broker::start().await else {
        return;
    };

    let session = open_test(port, false).await;
    let written = crate::engines::db::query(&session, "publish gpql/sensors hello")
        .await
        .unwrap();

    assert_eq!(written.affected, Some(1));

    let found = listed(&session, "gpql/sensors", 1).await;

    assert!(found
        .iter()
        .any(|table| table.name == "gpql/sensors" && table.rows >= 1));

    let page = table_rows(&session, "gpql/sensors", &Slice::default())
        .await
        .unwrap();

    assert!(
        page.rows
            .iter()
            .any(|row| row[0].as_deref() == Some("hello")),
        "{:?}",
        page.rows
    );
}
