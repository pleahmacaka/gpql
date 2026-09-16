use super::*;

use std::time::Duration;

use crate::engines::db::{open, query, Session, SessionConfig};
use crate::engines::slicing::table_rows;

// end to end against a real endpoint: GPQL_TEST_S3_URL (+USER/+SECRET) points
// at an existing one, otherwise a local rustfs is spawned when it is on PATH
static STORE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Endpoint {
    url: String,
    user: String,
    secret: String,
    _store: Option<Store>,
}

struct Store(std::process::Child);

impl Endpoint {
    async fn start() -> Option<Endpoint> {
        if let Ok(url) = std::env::var("GPQL_TEST_S3_URL") {
            return Some(Endpoint {
                url,
                user: std::env::var("GPQL_TEST_S3_USER").unwrap_or_default(),
                secret: std::env::var("GPQL_TEST_S3_SECRET").unwrap_or_default(),
                _store: None,
            });
        }

        if std::process::Command::new("rustfs")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!(
                "rustfs is not installed and GPQL_TEST_S3_URL is unset; \
                 skipping the live s3 test"
            );
            return None;
        }

        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let root = std::env::temp_dir().join(format!("gpql-rustfs-{port}"));

        // rustfs fails with VolumeNotFound unless the volume dir already exists
        std::fs::create_dir_all(&root).ok()?;

        let child = std::process::Command::new("rustfs")
            .args([
                "server",
                root.to_str().unwrap(),
                "--address",
                &format!("127.0.0.1:{port}"),
                "--access-key",
                "gpqltest",
                "--secret-key",
                "gpqlsecret",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;

        let store = Store(child);

        for _ in 0..100 {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                tokio::time::sleep(Duration::from_millis(500)).await;

                return Some(Endpoint {
                    url: format!("http://127.0.0.1:{port}"),
                    user: "gpqltest".to_string(),
                    secret: "gpqlsecret".to_string(),
                    _store: Some(store),
                });
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        drop(store);
        panic!("rustfs did not open its port");
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn open_test(endpoint: &Endpoint, read_only: bool) -> Session {
    let config = SessionConfig {
        kind: "s3".into(),
        url: endpoint.url.clone(),
        user: endpoint.user.clone(),
        password: endpoint.secret.clone(),
        read_only,
        ..Default::default()
    };

    for _ in 0..20 {
        match open(&config).await {
            Ok(session) => return session,
            Err(_) => tokio::time::sleep(Duration::from_millis(300)).await,
        }
    }

    panic!("could not open the test endpoint");
}

async fn run(session: &Session, statement: &str) -> QueryResult {
    query(session, statement)
        .await
        .unwrap_or_else(|error| panic!("`{statement}` failed: {error}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn buckets_and_objects_round_trip() {
    let _guard = STORE_LOCK.lock().await;
    let Some(endpoint) = Endpoint::start().await else {
        return;
    };

    let session = open_test(&endpoint, false).await;

    run(&session, "mk gpql-test").await;

    let buckets = run(&session, "ls").await;
    assert!(
        buckets
            .rows
            .iter()
            .any(|row| row[0].as_deref() == Some("gpql-test")),
        "{:?}",
        buckets.rows
    );

    run(&session, "put gpql-test/hello.txt hello gpql").await;
    run(&session, "put gpql-test/data/nodes.json {\"a\": 1}").await;

    let objects = run(&session, "ls gpql-test").await;
    let keys: Vec<String> = objects
        .rows
        .iter()
        .filter_map(|row| row[0].clone())
        .collect();

    assert!(keys.contains(&"hello.txt".to_string()), "{keys:?}");
    assert!(keys.contains(&"data/nodes.json".to_string()), "{keys:?}");

    // the grid path serves the same listing through table_rows
    let page = table_rows(&session, "gpql-test", &Slice::default())
        .await
        .unwrap();
    assert_eq!(page.columns, COLUMNS.map(|name| name.to_string()));
    assert_eq!(page.rows.len(), 2);

    let got = run(&session, "get gpql-test/hello.txt").await;
    assert_eq!(got.rows[0][0].as_deref(), Some("hello gpql"));

    let signed = run(&session, "presign gpql-test/hello.txt").await;
    assert!(signed.rows[0][0]
        .as_deref()
        .unwrap_or_default()
        .contains("hello.txt"));

    run(&session, "rm gpql-test/hello.txt").await;

    let objects = run(&session, "ls gpql-test").await;
    assert_eq!(objects.rows.len(), 1);

    run(&session, "rm gpql-test/data/nodes.json").await;
    run(&session, "rmb gpql-test").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_only_session_refuses_writes() {
    let _guard = STORE_LOCK.lock().await;
    let Some(endpoint) = Endpoint::start().await else {
        return;
    };

    let session = open_test(&endpoint, true).await;

    for statement in [
        "mk gpql-ro",
        "put gpql-test/x.txt nope",
        "rm gpql-test/x.txt",
        "rmb gpql-test",
    ] {
        let failure = query(&session, statement).await;
        let error = failure.err().unwrap_or_default();
        assert!(
            error.contains("read only"),
            "{statement} should be refused: {error}"
        );
    }
}
