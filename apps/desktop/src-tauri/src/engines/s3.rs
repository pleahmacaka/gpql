use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use s3::bucket::Bucket;
use s3::command::Command;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;
use s3::request::tokio_backend::ReqwestRequest;
use s3::request::{Request, ResponseData, ResponseDataStream};
use s3::serde_types::Object;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::engines::db::{QueryResult, SessionConfig, TableInfo};
use crate::engines::mqtt::window;
use crate::engines::slicing::{Op, Slice};

// listing a bucket walks 1000 keys per request, so the cache stops far short
// of unbounded memory while still covering realistic developer buckets
const LIST_CAP: usize = 20_000;
const PAGE_KEYS: usize = 1000;
const PRESIGN_SECS: u32 = 3600;
const GET_CAP: usize = 64 * 1024;
const SAID_CAP: usize = 300;

const COLUMNS: [&str; 5] = ["key", "bytes", "modified", "etag", "class"];

type Notify = Box<dyn Fn(&str) + Send>;
type Listings = HashMap<(String, String), Arc<Vec<Obj>>>;

#[derive(Clone)]
struct Obj {
    key: String,
    size: u64,
    modified: String,
    etag: String,
    class: String,
}

impl Obj {
    fn row(&self) -> Vec<Option<String>> {
        vec![
            Some(self.key.clone()),
            Some(self.size.to_string()),
            Some(self.modified.clone()),
            Some(self.etag.clone()),
            Some(self.class.clone()),
        ]
    }
}

impl From<&Object> for Obj {
    fn from(object: &Object) -> Self {
        Obj {
            key: object.key.clone(),
            size: object.size,
            modified: object.last_modified.clone(),
            etag: object.e_tag.clone().unwrap_or_default(),
            class: object.storage_class.clone().unwrap_or_default(),
        }
    }
}

pub struct S3 {
    region: Region,
    credentials: Credentials,
    custom: bool,
    only: Option<String>,
    clients: Mutex<HashMap<String, Bucket>>,
    objects: Mutex<Listings>,
    notify: Mutex<Option<Notify>>,
}

impl S3 {
    pub async fn open(config: &SessionConfig) -> Result<Self, String> {
        let name = if config.schema.is_empty() {
            "us-east-1"
        } else {
            config.schema.as_str()
        };

        let custom = !config.url.is_empty();

        let region = if custom {
            Region::Custom {
                region: name.to_string(),
                endpoint: config.url.clone(),
            }
        } else {
            aws(name)
        };

        let credentials = if config.user.is_empty() {
            Credentials::anonymous()
        } else {
            Credentials::new(
                Some(config.user.as_str()),
                Some(config.password.as_str()),
                None,
                None,
                None,
            )
        }
        .map_err(|error| error.to_string())?;

        let s3 = S3 {
            region,
            credentials,
            custom,
            only: if config.database.is_empty() {
                None
            } else {
                Some(config.database.clone())
            },
            clients: Mutex::new(HashMap::new()),
            objects: Mutex::new(HashMap::new()),
            notify: Mutex::new(None),
        };

        // the handshake names a dead or refusing endpoint here instead of
        // surfacing later as an empty bucket list
        let answer = match &s3.only {
            Some(name) => send(&s3.client(name).await?, "/", peek()).await?,
            None => send(&s3.lister()?, "", Command::ListBuckets).await?,
        };

        checked(answer)?;

        Ok(s3)
    }

    pub fn on_catalog_change(&self, notify: Notify) {
        *self.notify.lock().unwrap() = Some(notify);
    }

    fn changed(&self, bucket: &str) {
        if let Some(notify) = self.notify.lock().unwrap().as_ref() {
            notify(bucket);
        }
    }

    fn build(&self, name: &str, region: Region) -> Result<Bucket, String> {
        let mut bucket = *Bucket::new(name, region, self.credentials.clone())
            .map_err(|error| error.to_string())?;

        // minio and friends resolve <bucket>.<host> poorly, so off-aws endpoints
        // always take the path form
        if self.custom {
            bucket.set_path_style();
        }

        Ok(bucket)
    }

    fn lister(&self) -> Result<Bucket, String> {
        Ok(
            *Bucket::new("", self.region.clone(), self.credentials.clone())
                .map_err(|error| error.to_string())?
                .with_path_style(),
        )
    }

    // aws names a bucket's real region when asked in the wrong one; signing must follow
    async fn client(&self, name: &str) -> Result<Bucket, String> {
        let held = self.clients.lock().unwrap().get(name).cloned();

        if let Some(bucket) = held {
            return Ok(bucket);
        }

        let mut bucket = self.build(name, self.region.clone())?;

        if !self.custom {
            let moved = send(&bucket, "/", peek())
                .await
                .ok()
                .and_then(|answer| home(&answer));

            if let Some(region) = moved.filter(|region| *region != self.region) {
                bucket = self.build(name, region)?;
            }
        }

        self.clients
            .lock()
            .unwrap()
            .insert(name.to_string(), bucket.clone());

        Ok(bucket)
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let names = match &self.only {
            Some(name) => vec![name.clone()],
            None => match Bucket::list_buckets(self.region.clone(), self.credentials.clone()).await
            {
                Ok(listed) => listed.bucket_names().collect(),
                Err(error) => {
                    return Err(explain(&self.lister()?, "", Command::ListBuckets, error).await)
                }
            },
        };

        let objects = self.objects.lock().unwrap();

        let mut out: Vec<TableInfo> = names
            .into_iter()
            .map(|name| TableInfo {
                rows: objects
                    .get(&(name.clone(), String::new()))
                    .map_or(0, |list| list.len() as i64),
                name,
            })
            .collect();

        out.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(out)
    }

    async fn listing(&self, name: &str, prefix: &str) -> Result<Arc<Vec<Obj>>, String> {
        let slot = (name.to_string(), prefix.to_string());
        let held = self.objects.lock().unwrap().get(&slot).cloned();

        if let Some(objects) = held {
            return Ok(objects);
        }

        let bucket = self.client(name).await?;
        let mut objects: Vec<Obj> = Vec::new();
        let mut token: Option<String> = None;

        loop {
            let listed = bucket
                .list_page(
                    prefix.to_string(),
                    None,
                    token.clone(),
                    None,
                    Some(PAGE_KEYS),
                )
                .await;

            let page = match listed {
                Ok((page, _)) => page,
                Err(error) => {
                    let command = Command::ListObjectsV2 {
                        prefix: prefix.to_string(),
                        delimiter: None,
                        continuation_token: token,
                        start_after: None,
                        max_keys: Some(PAGE_KEYS),
                    };

                    return Err(explain(&bucket, "/", command, error).await);
                }
            };

            objects.extend(page.contents.iter().map(Obj::from));
            token = page.next_continuation_token;

            if token.is_none() || objects.len() >= LIST_CAP {
                break;
            }
        }

        let objects = Arc::new(objects);

        {
            let mut cache = self.objects.lock().unwrap();

            if !prefix.is_empty() {
                cache.retain(|(bucket, kept), _| bucket != name || kept.is_empty());
            }

            cache.insert(slot, objects.clone());
        }

        // announcing a narrowed listing would reload the grid into another listing
        if prefix.is_empty() {
            self.changed(name);
        }

        Ok(objects)
    }

    pub async fn page(&self, bucket: &str, slice: &Slice) -> Result<QueryResult, String> {
        let prefix = slice
            .filters
            .iter()
            .find(|filter| filter.column == "key" && matches!(filter.op, Op::Starts))
            .map_or("", |filter| filter.value.as_str());

        let objects = self.listing(bucket, prefix).await?;
        let rows = window(objects.iter().map(Obj::row).collect(), &COLUMNS, slice)?;

        Ok(QueryResult {
            columns: COLUMNS.iter().map(|name| name.to_string()).collect(),
            rows,
            affected: None,
        })
    }

    pub fn columns(&self) -> QueryResult {
        let objects = self.objects.lock().unwrap();
        let mut names: Vec<&String> = objects
            .keys()
            .filter(|(_, prefix)| prefix.is_empty())
            .map(|(name, _)| name)
            .collect();
        let mut rows = Vec::new();

        names.sort();

        for name in names {
            for column in COLUMNS {
                rows.push(vec![
                    Some(name.clone()),
                    Some(column.to_string()),
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

    fn forget(&self, bucket: &str) {
        self.objects
            .lock()
            .unwrap()
            .retain(|(name, _), _| name != bucket);
        self.changed(bucket);
    }

    async fn refresh_listing(&self, bucket: &str) -> Result<Arc<Vec<Obj>>, String> {
        self.forget(bucket);

        self.listing(bucket, "").await
    }

    pub async fn refresh(&self, bucket: &str) -> Result<(), String> {
        self.refresh_listing(bucket).await?;

        Ok(())
    }

    pub async fn presign(&self, bucket: &str, key: &str) -> Result<String, String> {
        // rust-s3 panics when it presigns without a secret key
        if self.credentials.secret_key.is_none() {
            return Err("presigning needs an access key and a secret key".to_string());
        }

        self.client(bucket)
            .await?
            .presign_get(object(key), PRESIGN_SECS, None)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn download(&self, bucket: &str, key: &str, path: &str) -> Result<u64, String> {
        let mut stream = self
            .client(bucket)
            .await?
            .get_object_stream(object(key))
            .await
            .map_err(|error| error.to_string())?;

        if !succeeded(stream.status_code) {
            let mut body = Vec::new();
            let _ = stream.read_to_end(&mut body).await;

            return Err(refusal(stream.status_code, &String::from_utf8_lossy(&body)));
        }

        let part = format!("{path}.part");

        let saved = match save(&mut stream, &part).await {
            Ok(size) => tokio::fs::rename(&part, path)
                .await
                .map(|_| size)
                .map_err(|error| error.to_string()),
            Err(error) => Err(error),
        };

        if saved.is_err() {
            let _ = tokio::fs::remove_file(&part).await;
        }

        saved
    }

    pub async fn upload(&self, bucket: &str, key: &str, path: &str) -> Result<(), String> {
        let body = tokio::fs::read(path)
            .await
            .map_err(|error| error.to_string())?;

        let response = self
            .client(bucket)
            .await?
            .put_object(object(key), &body)
            .await
            .map_err(|error| error.to_string())?;

        checked(response)?;
        self.forget(bucket);

        Ok(())
    }

    pub async fn delete(&self, bucket: &str, key: &str) -> Result<(), String> {
        let response = self
            .client(bucket)
            .await?
            .delete_object(object(key))
            .await
            .map_err(|error| error.to_string())?;

        checked(response)?;
        self.forget(bucket);

        Ok(())
    }

    pub async fn query(&self, sql: &str) -> Result<QueryResult, String> {
        let rest = sql.trim_start();
        let (verb, rest) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let verb = verb.to_ascii_lowercase();

        if matches!(verb.as_str(), "ls" | "list") {
            let wanted = rest.trim();

            if wanted.is_empty() {
                let rows = self
                    .tables()
                    .await?
                    .into_iter()
                    .map(|info| vec![Some(info.name)])
                    .collect();

                return Ok(QueryResult {
                    columns: vec!["bucket".to_string()],
                    rows,
                    affected: None,
                });
            }

            let objects = self.refresh_listing(wanted).await?;
            let rows = objects.iter().map(Obj::row).collect();

            return Ok(QueryResult {
                columns: COLUMNS.iter().map(|name| name.to_string()).collect(),
                rows,
                affected: None,
            });
        }

        if matches!(verb.as_str(), "mk" | "rmb") {
            let bucket = rest.trim();

            if bucket.is_empty() {
                return Err(format!("{verb} needs a bucket name"));
            }

            if verb == "mk" {
                // create builds its own client, so an off-aws endpoint needs
                // the path-style variant or it signs a bogus
                // <bucket>.127.0.0.1 host
                let setup = s3::bucket_ops::BucketConfiguration::default();
                let response = if self.custom {
                    Bucket::create_with_path_style(
                        bucket,
                        self.region.clone(),
                        self.credentials.clone(),
                        setup,
                    )
                    .await
                } else {
                    Bucket::create(bucket, self.region.clone(), self.credentials.clone(), setup)
                        .await
                }
                .map_err(|error| error.to_string())?;

                if !succeeded(response.response_code) {
                    return Err(refusal(response.response_code, &response.response_text));
                }
            } else {
                let client = self.client(bucket).await?;

                checked(send(&client, "", Command::DeleteBucket).await?)?;
            }

            self.forget(bucket);

            return Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected: Some(1),
            });
        }

        if !matches!(verb.as_str(), "get" | "presign" | "put" | "rm") {
            return Err("s3 speaks ls, mk, rmb, get, presign, put and rm".to_string());
        }

        let rest = rest.trim_start();

        // put carries a payload after the path, so its key stops at the first
        // space; the other verbs treat the whole tail as the path
        let (path, payload) = if verb == "put" {
            rest.split_once(char::is_whitespace)
                .map(|(path, body)| (path, body.trim()))
                .unwrap_or((rest, ""))
        } else {
            (rest, "")
        };

        let (bucket, key) = path.split_once('/').unwrap_or(("", ""));

        if bucket.is_empty() || key.is_empty() {
            return Err(format!("{verb} needs a <bucket>/<key>"));
        }

        match verb.as_str() {
            "get" => {
                let response = self
                    .client(bucket)
                    .await?
                    .get_object_range(object(key), 0, Some(GET_CAP as u64))
                    .await
                    .map_err(|error| error.to_string())?;

                let response = checked(response)?;
                let body = response.bytes();
                let content = match std::str::from_utf8(body) {
                    Ok(text) => text.to_string(),
                    Err(_) => format!("{} bytes", body.len()),
                };

                return Ok(QueryResult {
                    columns: vec!["content".to_string()],
                    rows: vec![vec![Some(content)]],
                    affected: None,
                });
            }
            "presign" => {
                let url = self.presign(bucket, key).await?;

                return Ok(QueryResult {
                    columns: vec!["url".to_string()],
                    rows: vec![vec![Some(url)]],
                    affected: None,
                });
            }
            "put" => {
                if payload.is_empty() {
                    return Err("put needs <bucket>/<key> and a payload".to_string());
                }

                let response = self
                    .client(bucket)
                    .await?
                    .put_object(object(key), payload.as_bytes())
                    .await
                    .map_err(|error| error.to_string())?;

                checked(response)?;
                self.forget(bucket);
            }
            _ => {
                self.delete(bucket, key).await?;
            }
        }

        Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected: Some(1),
        })
    }
}

// rust-s3 strips one leading slash from every key
fn object(key: &str) -> String {
    format!("/{key}")
}

fn peek() -> Command<'static> {
    Command::ListObjectsV2 {
        prefix: String::new(),
        delimiter: None,
        continuation_token: None,
        start_after: None,
        max_keys: Some(1),
    }
}

fn aws(name: &str) -> Region {
    match name.parse::<Region>() {
        Ok(Region::Custom { .. }) | Err(_) => Region::Custom {
            region: name.to_string(),
            endpoint: format!("https://s3.{name}.amazonaws.com"),
        },
        Ok(region) => region,
    }
}

async fn send(bucket: &Bucket, path: &str, command: Command<'_>) -> Result<ResponseData, String> {
    ReqwestRequest::new(bucket, path, command)
        .await
        .map_err(|error| error.to_string())?
        .response_data(false)
        .await
        .map_err(|error| error.to_string())
}

// rust-s3 parses before checking status, so an error reply surfaces as an xml failure
async fn explain(bucket: &Bucket, path: &str, command: Command<'_>, error: S3Error) -> String {
    if !matches!(error, S3Error::SerdeXml(_)) {
        return error.to_string();
    }

    match send(bucket, path, command).await {
        Ok(answer) if !succeeded(answer.status_code()) => refused(&answer),
        _ => error.to_string(),
    }
}

async fn save(stream: &mut ResponseDataStream, path: &str) -> Result<u64, String> {
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|error| error.to_string())?;

    let size = tokio::io::copy(stream, &mut file)
        .await
        .map_err(|error| error.to_string())?;

    file.flush().await.map_err(|error| error.to_string())?;

    Ok(size)
}

fn succeeded(status: u16) -> bool {
    (200..300).contains(&status)
}

fn checked(answer: ResponseData) -> Result<ResponseData, String> {
    if succeeded(answer.status_code()) {
        Ok(answer)
    } else {
        Err(refused(&answer))
    }
}

fn refused(answer: &ResponseData) -> String {
    refusal(
        answer.status_code(),
        &String::from_utf8_lossy(answer.as_slice()),
    )
}

fn refusal(status: u16, body: &str) -> String {
    match (element(body, "Code"), element(body, "Message")) {
        (Some(code), Some(message)) => format!("{code}: {message}"),
        (Some(code), None) => code.to_string(),
        _ if body.trim().is_empty() => format!("the endpoint answered {status}"),
        _ => format!(
            "the endpoint answered {status}: {}",
            body.trim().chars().take(SAID_CAP).collect::<String>()
        ),
    }
}

fn home(answer: &ResponseData) -> Option<Region> {
    let named = answer
        .headers()
        .get("x-amz-bucket-region")
        .cloned()
        .or_else(|| {
            element(&String::from_utf8_lossy(answer.as_slice()), "Region").map(str::to_string)
        })?;

    Some(aws(&named))
}

fn element<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let start = body.find(&open)? + open.len();
    let end = start + body.get(start..)?.find(&format!("</{name}>"))?;

    body.get(start..end).map(str::trim)
}

#[cfg(test)]
#[path = "s3_tests.rs"]
mod s3_tests;
