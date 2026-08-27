use std::collections::HashMap;

use rusqlite::Connection;

use super::db::{
    query_postgres, query_sqlite, quote_ident, ColumnInfo, Engine, QueryResult,
    Session, TableInfo, TableSchema,
};

pub async fn tables(session: &Session) -> Result<Vec<TableInfo>, String> {
    match &session.engine {
        Engine::Postgres(client) => {
            let result = query_postgres(
                client,
                "select c.relname, coalesce(s.n_live_tup, 0)::text
                 from pg_class c
                 join pg_namespace n on n.oid = c.relnamespace
                 left join pg_stat_user_tables s on s.relid = c.oid
                 where c.relkind = 'r' and n.nspname = current_schema()
                 order by c.relname",
            )
            .await?;

            return Ok(result
                .rows
                .into_iter()
                .map(|row| TableInfo {
                    name: row[0].clone().unwrap_or_default(),
                    rows: row[1].as_deref().unwrap_or("0").parse().unwrap_or(0),
                })
                .collect());
        }
        Engine::Sqlite(connection) => {
            let connection = connection.lock().unwrap();
            let names = query_sqlite(
                &connection,
                "select name from sqlite_master
                 where type = 'table' and name not like 'sqlite_%'
                 order by name",
            )?;

            let mut out = Vec::new();

            for row in names.rows {
                let name = row[0].clone().unwrap_or_default();
                let counted = query_sqlite(
                    &connection,
                    &format!("select count(*) from {}", quote_ident(&name)),
                )?;
                let rows = counted.rows[0][0]
                    .as_deref()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);

                out.push(TableInfo { name, rows });
            }

            return Ok(out);
        }
        Engine::MySql(client) => client.tables().await,
        Engine::Duck(duck) => duck.tables(),
        Engine::Http(remote) => remote.tables().await,
        Engine::Driver(driver) => driver.tables().await,
        Engine::Graph(graph) => graph.tables().await,
    }
}

pub async fn schema(session: &Session) -> Result<Vec<TableSchema>, String> {
    let counts: HashMap<String, i64> = tables(session)
        .await?
        .into_iter()
        .map(|table| (table.name, table.rows))
        .collect();

    let mut out = match &session.engine {
        Engine::Postgres(client) => {
            let mut tables = postgres_schema(client).await?;
            let notes = postgres_notes(client).await.unwrap_or_default();

            for (table, column, raw) in notes {
                let (text, hints) = annotation(&raw);

                let Some(found) = tables.iter_mut().find(|entry| entry.name == table)
                else {
                    continue;
                };

                if column.is_empty() {
                    found.note = text.or(Some(raw));
                    found.hints = hints;
                    continue;
                }

                if let Some(target) =
                    found.columns.iter_mut().find(|entry| entry.name == column)
                {
                    target.note = text.or(Some(raw));
                    found.hints.extend(hints);
                }
            }

            for (table, line) in postgres_guards(client).await? {
                if let Some(found) =
                    tables.iter_mut().find(|entry| entry.name == table)
                {
                    found.policies.push(line);
                }
            }

            tables
        }
        Engine::Sqlite(connection) => sqlite_schema(&connection.lock().unwrap(), &counts)?,
        Engine::MySql(client) => mysql_schema(client.columns().await?),
        Engine::Http(remote) if remote.flavour == "supabase_api" => {
            mysql_schema(remote.columns().await?)
        }
        Engine::Duck(duck) => mysql_schema(duck.columns()?),
        _ => counts
            .keys()
            .map(|name| TableSchema {
                name: name.clone(),
                ..Default::default()
            })
            .collect(),
    };

    for table in &mut out {
        table.rows = *counts.get(&table.name).unwrap_or(&0);
    }

    return Ok(out);
}

pub async fn schemas(session: &Session) -> Result<Vec<String>, String> {
    match &session.engine {
        Engine::Postgres(client) => {
            let result = query_postgres(
                client,
                "select nspname from pg_namespace
                 where nspname not like 'pg\\_%'
                   and nspname <> 'information_schema'
                 order by nspname",
            )
            .await?;

            return Ok(result
                .rows
                .into_iter()
                .filter_map(|row| row[0].clone())
                .collect());
        }
        // every other engine exposes a single schema, so there is nothing to pick
        _ => Ok(Vec::new()),
    }
}

pub async fn use_schema(session: &Session, name: &str) -> Result<(), String> {
    match &session.engine {
        Engine::Postgres(client) => {
            query_postgres(
                client,
                &format!("set search_path to {}", quote_ident(name)),
            )
            .await?;

            return Ok(());
        }
        _ => Err("this engine has a single schema".into()),
    }
}

async fn postgres_guards(
    client: &tokio_postgres::Client,
) -> Result<Vec<(String, String)>, String> {
    let policies = query_postgres(
        client,
        "select tablename, policyname, coalesce(cmd, 'ALL')
         from pg_policies where schemaname = current_schema()",
    )
    .await?;
    let rules = query_postgres(
        client,
        "select tablename, rulename from pg_rules
         where schemaname = current_schema() and rulename <> '_RETURN'",
    )
    .await?;

    let mut out = Vec::new();

    for row in policies.rows {
        out.push((
            row[0].clone().unwrap_or_default(),
            format!(
                "policy {} ({})",
                row[1].clone().unwrap_or_default(),
                row[2].clone().unwrap_or_default()
            ),
        ));
    }

    for row in rules.rows {
        out.push((
            row[0].clone().unwrap_or_default(),
            format!("rule {}", row[1].clone().unwrap_or_default()),
        ));
    }

    return Ok(out);
}

async fn postgres_schema(client: &tokio_postgres::Client) -> Result<Vec<TableSchema>, String> {
    let columns = query_postgres(
        client,
        "select table_name, column_name, data_type, is_nullable
         from information_schema.columns
         where table_schema = current_schema()
         order by table_name, ordinal_position",
    )
    .await?;

    let keys = query_postgres(
        client,
        "select tc.constraint_type, kcu.table_name, kcu.column_name,
                ccu.table_name as target_table, ccu.column_name as target_column
         from information_schema.table_constraints tc
         join information_schema.key_column_usage kcu
           on kcu.constraint_name = tc.constraint_name
          and kcu.table_schema = tc.table_schema
         left join information_schema.constraint_column_usage ccu
           on ccu.constraint_name = tc.constraint_name
          and ccu.table_schema = tc.table_schema
         where tc.table_schema = current_schema()
           and tc.constraint_type in ('PRIMARY KEY', 'FOREIGN KEY')",
    )
    .await?;

    let mut primary = std::collections::HashSet::new();
    let mut foreign: HashMap<(String, String), String> = HashMap::new();

    for row in keys.rows {
        let kind = row[0].clone().unwrap_or_default();
        let table = row[1].clone().unwrap_or_default();
        let column = row[2].clone().unwrap_or_default();

        if kind == "PRIMARY KEY" {
            primary.insert((table, column));
            continue;
        }

        let target_table = row[3].clone().unwrap_or_default();
        let target_column = row[4].clone().unwrap_or_default();

        foreign.insert((table, column), format!("{target_table}.{target_column}"));
    }

    let mut grouped: Vec<TableSchema> = Vec::new();

    for row in columns.rows {
        let table = row[0].clone().unwrap_or_default();
        let name = row[1].clone().unwrap_or_default();
        let data_type = row[2].clone().unwrap_or_default();
        let required = row[3].as_deref() == Some("NO");

        if grouped.last().map(|last| last.name != table).unwrap_or(true) {
            grouped.push(TableSchema {
                name: table.clone(),
                ..Default::default()
            });
        }

        grouped.last_mut().unwrap().columns.push(ColumnInfo {
            primary_key: primary.contains(&(table.clone(), name.clone())),
            references: foreign.get(&(table.clone(), name.clone())).cloned(),
            name,
            data_type,
            required,
            note: None,
        });
    }

    return Ok(grouped);
}

pub fn annotation(raw: &str) -> (Option<String>, Vec<String>) {
    let Some(start) = raw.find("@gpql:comment") else {
        return (None, Vec::new());
    };

    let rest = &raw[start + "@gpql:comment".len()..];
    let quoted = rest
        .split_once('"')
        .and_then(|(_, tail)| tail.split_once('"'))
        .map(|(body, _)| body.trim().to_string());

    let Some(text) = quoted else {
        return (None, Vec::new());
    };

    let hints = text
        .split_whitespace()
        .filter_map(|word| word.strip_prefix("@ref"))
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '.'))
        .filter(|word| word.contains('.'))
        .map(str::to_string)
        .collect::<Vec<_>>();

    let extra = text
        .split_whitespace()
        .enumerate()
        .filter(|(index, word)| {
            !word.starts_with("@ref")
                && !(*index > 0 && text.split_whitespace().nth(index - 1) == Some("@ref"))
        })
        .map(|(_, word)| word)
        .collect::<Vec<_>>()
        .join(" ");

    return (Some(extra.trim().to_string()), hints);
}

async fn postgres_notes(
    client: &tokio_postgres::Client,
) -> Result<Vec<(String, String, String)>, String> {
    let listing = query_postgres(
        client,
        "select c.relname, coalesce(a.attname, ''), d.description
         from pg_description d
         join pg_class c on c.oid = d.objoid
         join pg_namespace n on n.oid = c.relnamespace
         left join pg_attribute a
           on a.attrelid = c.oid and a.attnum = d.objsubid
         where n.nspname = current_schema()",
    )
    .await?;

    return Ok(listing
        .rows
        .into_iter()
        .map(|row| {
            (
                row[0].clone().unwrap_or_default(),
                row[1].clone().unwrap_or_default(),
                row[2].clone().unwrap_or_default(),
            )
        })
        .collect());
}

fn mysql_schema(listing: QueryResult) -> Vec<TableSchema> {
    let mut grouped: Vec<TableSchema> = Vec::new();

    for row in listing.rows {
        let cell = |index: usize| row.get(index).cloned().flatten().unwrap_or_default();
        let table = cell(0);

        if grouped.last().map(|last| last.name != table).unwrap_or(true) {
            grouped.push(TableSchema {
                name: table.clone(),
                ..Default::default()
            });
        }

        let target = cell(5);

        grouped.last_mut().unwrap().columns.push(ColumnInfo {
            name: cell(1),
            data_type: cell(2),
            required: cell(3) == "NO",
            primary_key: cell(4) == "PRI",
            note: None,
            references: if target.is_empty() {
                None
            } else {
                Some(format!("{target}.{}", cell(6)))
            },
        });
    }

    return grouped;
}

fn sqlite_schema(
    connection: &Connection,
    counts: &HashMap<String, i64>,
) -> Result<Vec<TableSchema>, String> {
    let mut names: Vec<&String> = counts.keys().collect();
    names.sort();

    let mut out = Vec::new();

    for name in names {
        let info = query_sqlite(
            connection,
            &format!("pragma table_info({})", quote_ident(name)),
        )?;
        let links = query_sqlite(
            connection,
            &format!("pragma foreign_key_list({})", quote_ident(name)),
        )?;

        let mut foreign: HashMap<String, String> = HashMap::new();

        for row in links.rows {
            let from = row[3].clone().unwrap_or_default();
            let target_table = row[2].clone().unwrap_or_default();
            let target_column = row[4].clone().unwrap_or_else(|| "rowid".into());

            foreign.insert(from, format!("{target_table}.{target_column}"));
        }

        let columns = info
            .rows
            .into_iter()
            .map(|row| {
                let column = row[1].clone().unwrap_or_default();

                ColumnInfo {
                    data_type: row[2].clone().unwrap_or_default().to_lowercase(),
                    required: row[3].as_deref() == Some("1"),
                    primary_key: row[5].as_deref().unwrap_or("0") != "0",
                    references: foreign.get(&column).cloned(),
                    note: None,
                    name: column,
                }
            })
            .collect();

        out.push(TableSchema {
            name: name.clone(),
            columns,
            ..Default::default()
        });
    }

    return Ok(out);
}
