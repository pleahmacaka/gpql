use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex};

use serde::Deserialize;

use super::db::{held, literal, quote_for, QueryResult, Session};
use super::slicing::{binary_columns, table_rows, Slice};

const PAGE: u32 = 5_000;

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Json,
    Sql,
}

struct Sink {
    out: Arc<Mutex<BufWriter<File>>>,
    format: Format,
    table: String,
    binary: HashSet<String>,
    written: u64,
}

fn csv_line(cells: impl Iterator<Item = Option<String>>) -> Result<String, String> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(Vec::new());

    writer
        .write_record(cells.map(|cell| cell.unwrap_or_default()))
        .map_err(|error| error.to_string())?;

    let line = writer.into_inner().map_err(|error| error.to_string())?;

    String::from_utf8(line).map_err(|error| error.to_string())
}

// the grid shows bytes as 0x hex, and each dialect spells a byte string
// literal its own way
fn bytes_literal(flavour: &str, hex: &str) -> String {
    match flavour {
        "snowflake" => format!("to_binary('{hex}', 'HEX')"),
        _ => format!("unhex('{hex}')"),
    }
}

impl Sink {
    async fn open(
        session: &Session,
        path: &str,
        format: Format,
        table: &str,
    ) -> Result<Self, String> {
        let binary = if format == Format::Sql {
            binary_columns(session, table).await.unwrap_or_default()
        } else {
            HashSet::new()
        };

        let path = path.to_string();
        let file = tokio::task::spawn_blocking(move || File::create(path))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;

        Ok(Sink {
            out: Arc::new(Mutex::new(BufWriter::new(file))),
            format,
            table: table.to_string(),
            binary,
            written: 0,
        })
    }

    async fn put(&self, text: String) -> Result<(), String> {
        let out = Arc::clone(&self.out);

        tokio::task::spawn_blocking(move || {
            held(&out)
                .write_all(text.as_bytes())
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())?
    }

    async fn head(&self, columns: &[String]) -> Result<(), String> {
        match self.format {
            Format::Csv => {
                self.put(csv_line(columns.iter().map(|name| Some(name.clone())))?)
                    .await
            }
            Format::Json => self.put("[\n".to_string()).await,
            Format::Sql => Ok(()),
        }
    }

    fn value(&self, session: &Session, column: &str, cell: &Option<String>) -> String {
        let hex = cell
            .as_deref()
            .and_then(|text| text.strip_prefix("0x"))
            .filter(|digits| digits.chars().all(|c| c.is_ascii_hexdigit()));

        match hex {
            Some(digits) if self.binary.contains(column) => bytes_literal(&session.flavour, digits),
            _ => literal(&session.flavour, cell),
        }
    }

    async fn body(&mut self, session: &Session, page: &QueryResult) -> Result<(), String> {
        let mut text = String::new();

        for row in &page.rows {
            match self.format {
                Format::Csv => text.push_str(&csv_line(row.iter().cloned())?),
                Format::Json => {
                    let pairs = page
                        .columns
                        .iter()
                        .zip(row)
                        .map(|(name, cell)| {
                            let value = match cell {
                                None => "null".to_string(),
                                Some(text) => serde_json::Value::String(text.clone()).to_string(),
                            };

                            format!("{}: {value}", serde_json::Value::String(name.clone()))
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    let lead = if self.written == 0 { "  {" } else { ",\n  {" };

                    text.push_str(&format!("{lead}{pairs}}}"));
                }
                Format::Sql => {
                    let names = page
                        .columns
                        .iter()
                        .map(|name| quote_for(session, name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let values = page
                        .columns
                        .iter()
                        .zip(row)
                        .map(|(name, cell)| self.value(session, name, cell))
                        .collect::<Vec<_>>()
                        .join(", ");

                    text.push_str(&format!(
                        "insert into {} ({names}) values ({values});\n",
                        quote_for(session, &self.table)
                    ));
                }
            }

            self.written += 1;
        }

        self.put(text).await
    }

    async fn finish(self) -> Result<u64, String> {
        if self.format == Format::Json {
            self.put("\n]\n".to_string()).await?;
        }

        let out = Arc::clone(&self.out);

        tokio::task::spawn_blocking(move || held(&out).flush().map_err(|error| error.to_string()))
            .await
            .map_err(|error| error.to_string())??;

        Ok(self.written)
    }
}

// walks the table a page at a time so a million rows never sit in memory at once
pub async fn export_table(
    session: &Session,
    table: &str,
    slice: &Slice,
    format: Format,
    path: &str,
) -> Result<u64, String> {
    let mut sink = Sink::open(session, path, format, table).await?;
    let mut offset = slice.offset;
    let mut headed = false;
    let ceiling = if slice.limit == 0 {
        u32::MAX
    } else {
        slice.limit
    };

    loop {
        let want = Slice {
            limit: PAGE.min(ceiling.saturating_sub(sink.written as u32)),
            offset,
            sort: slice.sort.clone(),
            filters: slice.filters.clone(),
            columns: slice.columns.clone(),
        };

        if want.limit == 0 {
            break;
        }

        let page = table_rows(session, table, &want).await?;

        if !headed {
            sink.head(&page.columns).await?;
            headed = true;
        }

        let count = page.rows.len() as u32;

        sink.body(session, &page).await?;

        if count < want.limit {
            break;
        }

        offset += count;
    }

    if !headed {
        sink.head(&[]).await?;
    }

    sink.finish().await
}

pub async fn export_result(
    session: &Session,
    result: &QueryResult,
    table: &str,
    format: Format,
    path: &str,
) -> Result<u64, String> {
    let mut sink = Sink::open(session, path, format, table).await?;

    sink.head(&result.columns).await?;
    sink.body(session, result).await?;

    sink.finish().await
}
