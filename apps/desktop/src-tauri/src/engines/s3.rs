use std::collections::HashMap;
use std::sync::Mutex;

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::serde_types::Object;

use crate::engines::db::{QueryResult, SessionConfig, TableInfo};
use crate::engines::slicing::Slice;

// listing a bucket walks 1000 keys per request, so the cache stops far short
// of unbounded memory while still covering realistic developer buckets
const LIST_CAP: usize = 20_000;
const PRESIGN_SECS: u32 = 3600;
const GET_CAP: usize = 64 * 1024;

const COLUMNS: [&str; 5] = ["key", "bytes", "modified", "etag", "class"];

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
        return vec![
            Some(self.key.clone()),
            Some(self.size.to_string()),
            Some(self.modified.clone()),
            Some(self.etag.clone()),
            Some(self.class.clone()),
        ];
    }
}

impl From<&Object> for Obj {
    fn from(object: &Object) -> Self {
        return Obj {
            key: object.key.clone(),
            size: object.size,
            modified: object.last_modified.clone(),
            etag: object.e_tag.clone().unwrap_or_default(),
            class: object.storage_class.clone().unwrap_or_default(),
        };
    }
}

pub struct S3 {
    region: Region,
    credentials: Credentials,
    custom: bool,
    only: Option<String>,
    clients: Mutex<HashMap<String, Bucket>>,
    objects: Mutex<HashMap<String, Vec<Obj>>>,
    notify: Mutex<Option<Box<dyn Fn(&str) + Send>>>,
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
            name.parse::<Region>()
                .map_err(|_| format!("unknown region {name}"))?
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
        match &s3.only {
            Some(name) => {
                let bucket = s3.client(name)?;

                if !bucket.exists().await.map_err(|error| error.to_string())? {
                    return Err(format!("no bucket named {name}"));
                }
            }
            None => {
                Bucket::list_buckets(s3.region.clone(), s3.credentials.clone())
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }

        return Ok(s3);
    }

    pub fn on_catalog_change(&self, notify: Box<dyn Fn(&str) + Send>) {
        *self.notify.lock().unwrap() = Some(notify);
    }

    fn changed(&self, bucket: &str) {
        if let Some(notify) = self.notify.lock().unwrap().as_ref() {
            notify(bucket);
        }
    }

    fn client(&self, name: &str) -> Result<Bucket, String> {
        if let Some(bucket) = self.clients.lock().unwrap().get(name) {
            return Ok(bucket.clone());
        }

        let mut bucket = *Bucket::new(name, self.region.clone(), self.credentials.clone())
            .map_err(|error| error.to_string())?;

        // minio and friends resolve <bucket>.<host> poorly, so off-aws endpoints
        // always take the path form
        if self.custom {
            bucket.set_path_style();
        }

        self.clients
            .lock()
            .unwrap()
            .insert(name.to_string(), bucket.clone());

        return Ok(bucket);
    }

    pub async fn tables(&self) -> Result<Vec<TableInfo>, String> {
        let names = match &self.only {
            Some(name) => vec![name.clone()],
            None => Bucket::list_buckets(self.region.clone(), self.credentials.clone())
                .await
                .map_err(|error| error.to_string())?
                .bucket_names()
                .collect(),
        };

        let objects = self.objects.lock().unwrap();

        let mut out: Vec<TableInfo> = names
            .into_iter()
            .map(|name| TableInfo {
                rows: objects
                    .get(&name)
                    .map(|list| list.len() as i64)
                    .unwrap_or(0),
                name,
            })
            .collect();

        out.sort_by(|a, b| a.name.cmp(&b.name));

        return Ok(out);
    }

    async fn listing(&self, name: &str) -> Result<Vec<Obj>, String> {
        if let Some(cached) = self.objects.lock().unwrap().get(name) {
            return Ok(cached.clone());
        }

        let bucket = self.client(name)?;
        let mut objects: Vec<Obj> = Vec::new();
        let mut token: Option<String> = None;

        loop {
            let (page, _) = bucket
                .list_page(String::new(), None, token, None, Some(1000))
                .await
                .map_err(|error| error.to_string())?;

            objects.extend(page.contents.iter().map(Obj::from));
            token = page.next_continuation_token;

            if token.is_none() || objects.len() >= LIST_CAP {
                break;
            }
        }

        self.objects
            .lock()
            .unwrap()
            .insert(name.to_string(), objects.clone());
        self.changed(name);

        return Ok(objects);
    }

    pub async fn page(&self, bucket: &str, slice: &Slice) -> Result<QueryResult, String> {
        let objects = self.listing(bucket).await?;
        let mut picked: Vec<&Obj> = objects.iter().collect();

        if let Some(sort) = &slice.sort {
            let index = COLUMNS
                .iter()
                .position(|name| *name == sort.column)
                .unwrap_or(0);

            picked.sort_by(|a, b| {
                let order = if index == 1 {
                    a.size.cmp(&b.size)
                } else {
                    a.row()[index].cmp(&b.row()[index])
                };

                return if sort.descending {
                    order.reverse()
                } else {
                    order
                };
            });
        }

        let window = picked.into_iter().skip(slice.offset as usize);
        let rows = if slice.limit == 0 {
            window.map(|object| object.row()).collect()
        } else {
            window
                .take(slice.limit as usize)
                .map(|object| object.row())
                .collect()
        };

        return Ok(QueryResult {
            columns: COLUMNS.iter().map(|name| name.to_string()).collect(),
            rows,
            affected: None,
        });
    }

    pub fn columns(&self) -> QueryResult {
        let objects = self.objects.lock().unwrap();
        let mut names: Vec<&String> = objects.keys().collect();
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

        return QueryResult {
            columns: Vec::new(),
            rows,
            affected: None,
        };
    }

    fn forget(&self, bucket: &str) {
        self.objects.lock().unwrap().remove(bucket);
        self.changed(bucket);
    }

    async fn refresh_listing(&self, bucket: &str) -> Result<Vec<Obj>, String> {
        self.forget(bucket);

        return self.listing(bucket).await;
    }

    pub async fn refresh(&self, bucket: &str) -> Result<(), String> {
        self.refresh_listing(bucket).await?;

        return Ok(());
    }

    pub async fn presign(&self, bucket: &str, key: &str) -> Result<String, String> {
        return self
            .client(bucket)?
            .presign_get(key, PRESIGN_SECS, None)
            .await
            .map_err(|error| error.to_string());
    }

    pub async fn download(&self, bucket: &str, key: &str, path: &str) -> Result<u64, String> {
        let mut file = tokio::fs::File::create(path)
            .await
            .map_err(|error| error.to_string())?;

        let status = self
            .client(bucket)?
            .get_object_to_writer(key, &mut file)
            .await
            .map_err(|error| error.to_string())?;

        if status != 200 {
            return Err(format!("the object answered {status}"));
        }

        return Ok(file.metadata().await.map_err(|e| e.to_string())?.len());
    }

    pub async fn upload(&self, bucket: &str, key: &str, path: &str) -> Result<(), String> {
        let body = tokio::fs::read(path)
            .await
            .map_err(|error| error.to_string())?;

        let response = self
            .client(bucket)?
            .put_object(key, &body)
            .await
            .map_err(|error| error.to_string())?;

        if !(200..300).contains(&response.status_code()) {
            return Err(format!("the endpoint answered {}", response.status_code()));
        }

        self.forget(bucket);

        return Ok(());
    }

    pub async fn delete(&self, bucket: &str, key: &str) -> Result<(), String> {
        let response = self
            .client(bucket)?
            .delete_object(key)
            .await
            .map_err(|error| error.to_string())?;

        if !(200..300).contains(&response.status_code()) {
            return Err(format!("the endpoint answered {}", response.status_code()));
        }

        self.forget(bucket);

        return Ok(());
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
            let rows = objects.iter().map(|object| object.row()).collect();

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

                if !(200..300).contains(&response.response_code) {
                    return Err(format!(
                        "the endpoint answered {}: {}",
                        response.response_code, response.response_text
                    ));
                }
            } else {
                let status = self
                    .client(bucket)?
                    .delete()
                    .await
                    .map_err(|error| error.to_string())?;

                if !(200..300).contains(&status) {
                    return Err(format!("the endpoint answered {status}"));
                }
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
                    .client(bucket)?
                    .get_object_range(key, 0, Some(GET_CAP as u64))
                    .await
                    .map_err(|error| error.to_string())?;

                let status = response.status_code();

                if status != 200 && status != 206 {
                    return Err(format!("the object answered {status}"));
                }

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
                    .client(bucket)?
                    .put_object(key, payload.as_bytes())
                    .await
                    .map_err(|error| error.to_string())?;

                if !(200..300).contains(&response.status_code()) {
                    return Err(format!("the endpoint answered {}", response.status_code()));
                }

                self.forget(bucket);
            }
            _ => {
                self.delete(bucket, key).await?;
            }
        }

        return Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected: Some(1),
        });
    }
}

#[cfg(test)]
#[path = "s3_tests.rs"]
mod s3_tests;
