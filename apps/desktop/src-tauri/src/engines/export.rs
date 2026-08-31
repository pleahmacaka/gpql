use std::fs::File;
use std::io::{BufWriter, Write};

use serde::Deserialize;

use super::db::{literal, quote_for, QueryResult, Session};
use super::slicing::{table_rows, Slice};

const PAGE: u32 = 5_000;

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Json,
    Sql,
}

struct Sink {
    out: BufWriter<File>,
    format: Format,
    table: String,
    written: u64,
}

impl Sink {
    fn open(path: &str, format: Format, table: &str) -> Result<Self, String> {
        let file = File::create(path).map_err(|error| error.to_string())?;

        return Ok(Sink {
            out: BufWriter::new(file),
            format,
            table: table.to_string(),
            written: 0,
        });
    }

    fn head(&mut self, columns: &[String]) -> Result<(), String> {
        return match self.format {
            Format::Csv => self.row_csv(columns.iter().map(|name| Some(name.clone()))),
            Format::Json => self.put("[\n"),
            Format::Sql => Ok(()),
        };
    }

    fn put(&mut self, text: &str) -> Result<(), String> {
        return self
            .out
            .write_all(text.as_bytes())
            .map_err(|error| error.to_string());
    }

    fn row_csv(
        &mut self,
        cells: impl Iterator<Item = Option<String>>,
    ) -> Result<(), String> {
        let mut writer = csv::WriterBuilder::new()
            .terminator(csv::Terminator::Any(b'\n'))
            .from_writer(Vec::new());

        writer
            .write_record(cells.map(|cell| cell.unwrap_or_default()))
            .map_err(|error| error.to_string())?;

        let line = writer.into_inner().map_err(|error| error.to_string())?;

        return self
            .out
            .write_all(&line)
            .map_err(|error| error.to_string());
    }

    fn body(
        &mut self,
        session: &Session,
        page: &QueryResult,
    ) -> Result<(), String> {
        for row in &page.rows {
            match self.format {
                Format::Csv => self.row_csv(row.iter().cloned())?,
                Format::Json => {
                    let pairs = page
                        .columns
                        .iter()
                        .zip(row)
                        .map(|(name, cell)| {
                            let value = match cell {
                                None => "null".to_string(),
                                Some(text) => serde_json::Value::String(text.clone())
                                    .to_string(),
                            };

                            format!("{}: {value}", serde_json::Value::String(name.clone()))
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    let lead = if self.written == 0 { "  {" } else { ",\n  {" };

                    self.put(&format!("{lead}{pairs}}}"))?;
                }
                Format::Sql => {
                    let names = page
                        .columns
                        .iter()
                        .map(|name| quote_for(session, name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let values = row
                        .iter()
                        .map(literal)
                        .collect::<Vec<_>>()
                        .join(", ");

                    self.put(&format!(
                        "insert into {} ({names}) values ({values});\n",
                        quote_for(session, &self.table)
                    ))?;
                }
            }

            self.written += 1;
        }

        return Ok(());
    }

    fn finish(mut self) -> Result<u64, String> {
        if self.format == Format::Json {
            self.put("\n]\n")?;
        }

        self.out.flush().map_err(|error| error.to_string())?;

        return Ok(self.written);
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
    let mut sink = Sink::open(path, format, table)?;
    let mut offset = slice.offset;
    let mut headed = false;
    let ceiling = if slice.limit == 0 { u32::MAX } else { slice.limit };

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
            sink.head(&page.columns)?;
            headed = true;
        }

        let count = page.rows.len() as u32;

        sink.body(session, &page)?;

        if count < want.limit {
            break;
        }

        offset += count;
    }

    if !headed {
        sink.head(&[])?;
    }

    return sink.finish();
}

pub fn export_result(
    session: &Session,
    result: &QueryResult,
    table: &str,
    format: Format,
    path: &str,
) -> Result<u64, String> {
    let mut sink = Sink::open(path, format, table)?;

    sink.head(&result.columns)?;
    sink.body(session, result)?;

    return sink.finish();
}
