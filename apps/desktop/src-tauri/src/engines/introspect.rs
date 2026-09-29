use std::collections::HashMap;

use rusqlite::Connection;

use super::db::{
    held, query_sqlite, quote_ident, read, with_sqlite, ColumnInfo, Engine, QueryResult, Session,
    TableInfo, TableSchema,
};

pub async fn tables(session: &Session) -> Result<Vec<TableInfo>, String> {
    match &session.engine {
        Engine::Postgres(_) => {
            // a partitioned table is relkind p, and its partitions are plain
            // tables that would otherwise be listed beside it
            let listing = if session.flavour == "greptimedb" {
                "select c.relname, coalesce(s.n_live_tup, 0)::text
                 from pg_class c
                 join pg_namespace n on n.oid = c.relnamespace
                 left join pg_stat_user_tables s on s.relid = c.oid
                 where c.relkind = 'r' and n.nspname = current_schema()
                 order by c.relname"
            } else {
                "select c.relname, coalesce(s.n_live_tup, 0)::text
                 from pg_class c
                 join pg_namespace n on n.oid = c.relnamespace
                 left join pg_stat_user_tables s on s.relid = c.oid
                 where c.relkind in ('r', 'p') and not c.relispartition
                   and n.nspname = current_schema()
                 order by c.relname"
            };

            let result = read(session, listing).await?;

            Ok(result
                .rows
                .into_iter()
                .map(|row| TableInfo {
                    name: row[0].clone().unwrap_or_default(),
                    rows: row[1].as_deref().unwrap_or("0").parse().unwrap_or(0),
                })
                .collect())
        }
        Engine::Sqlite(connection) => with_sqlite(connection, sqlite_tables).await,
        Engine::MySql(client) => client.tables().await,
        Engine::Duck(duck) => duck.tables().await,
        Engine::Http(remote) => remote.tables().await,
        Engine::Driver(driver) => driver.tables().await,
        Engine::Graph(graph) => graph.tables().await,
    }
}

fn sqlite_tables(connection: &Connection) -> Result<Vec<TableInfo>, String> {
    let names = query_sqlite(
        connection,
        "select name from sqlite_master
         where type = 'table' and name not like 'sqlite_%'
         order by name",
    )?;

    let mut out = Vec::new();

    for row in names.rows {
        let name = row[0].clone().unwrap_or_default();
        let counted = query_sqlite(
            connection,
            &format!("select count(*) from {}", quote_ident(&name)),
        )?;
        let rows = counted
            .rows
            .first()
            .and_then(|row| row.first().cloned().flatten())
            .and_then(|count| count.parse().ok())
            .unwrap_or(0);

        out.push(TableInfo { name, rows });
    }

    Ok(out)
}

pub async fn schema(session: &Session) -> Result<Vec<TableSchema>, String> {
    let counts: HashMap<String, i64> = tables(session)
        .await?
        .into_iter()
        .map(|table| (table.name, table.rows))
        .collect();

    let mut out = match &session.engine {
        Engine::Postgres(_) => {
            let mut tables = postgres_schema(session).await?;
            let notes = postgres_notes(session).await.unwrap_or_default();

            for (table, column, raw) in notes {
                let (text, hints) = annotation(&raw);

                let Some(found) = tables.iter_mut().find(|entry| entry.name == table) else {
                    continue;
                };

                if column.is_empty() {
                    found.note = text.or(Some(raw));
                    found.hints = hints;
                    continue;
                }

                if let Some(target) = found.columns.iter_mut().find(|entry| entry.name == column) {
                    target.note = text.or(Some(raw));
                    found.hints.extend(hints);
                }
            }

            for (table, line) in postgres_guards(session).await {
                if let Some(found) = tables.iter_mut().find(|entry| entry.name == table) {
                    found.policies.push(line);
                }
            }

            tables
        }
        Engine::Sqlite(connection) => {
            let names = counts.keys().cloned().collect::<Vec<_>>();

            with_sqlite(connection, move |connection| {
                sqlite_schema(connection, names)
            })
            .await?
        }
        Engine::MySql(client) => mysql_schema(client.columns().await?),
        Engine::Http(remote) if remote.flavour == "supabase_api" => {
            mysql_schema(remote.columns().await?)
        }
        Engine::Duck(duck) => mysql_schema(duck.columns().await?),
        Engine::Driver(driver) => match driver.columns().await? {
            Some(listing) => mysql_schema(listing),
            None => bare(&counts),
        },
        _ => bare(&counts),
    };

    for table in &mut out {
        table.rows = *counts.get(&table.name).unwrap_or(&0);
    }

    Ok(out)
}

pub async fn schemas(session: &Session) -> Result<Vec<String>, String> {
    match &session.engine {
        Engine::Postgres(_) => {
            let result = read(
                session,
                "select nspname from pg_namespace
                 where nspname not like 'pg\\_%'
                   and nspname <> 'information_schema'
                 order by nspname",
            )
            .await?;

            Ok(result
                .rows
                .into_iter()
                .filter_map(|row| row[0].clone())
                .collect())
        }
        // every other engine exposes a single schema, so there is nothing to pick
        _ => Ok(Vec::new()),
    }
}

pub async fn use_schema(session: &Session, name: &str) -> Result<(), String> {
    match &session.engine {
        Engine::Postgres(_) => {
            read(
                session,
                &format!("set search_path to {}", quote_ident(name)),
            )
            .await?;

            *held(&session.search_path) = Some(name.to_string());
            held(&session.order_keys).clear();

            Ok(())
        }
        _ => Err("this engine has a single schema".into()),
    }
}

// greptimedb and older servers have no pg_policies or pg_rules, and a missing
// catalog should cost the policy lines, not the whole schema
async fn postgres_guards(session: &Session) -> Vec<(String, String)> {
    let policies = read(
        session,
        "select tablename, policyname, coalesce(cmd, 'ALL')
         from pg_policies where schemaname = current_schema()",
    )
    .await
    .map(|result| result.rows)
    .unwrap_or_default();
    let rules = read(
        session,
        "select tablename, rulename from pg_rules
         where schemaname = current_schema() and rulename <> '_RETURN'",
    )
    .await
    .map(|result| result.rows)
    .unwrap_or_default();

    let mut out = Vec::new();

    for row in policies {
        out.push((
            row[0].clone().unwrap_or_default(),
            format!(
                "policy {} ({})",
                row[1].clone().unwrap_or_default(),
                row[2].clone().unwrap_or_default()
            ),
        ));
    }

    for row in rules {
        out.push((
            row[0].clone().unwrap_or_default(),
            format!("rule {}", row[1].clone().unwrap_or_default()),
        ));
    }

    out
}

// the key columns pair up by position in conkey and confkey, which is the
// only way a composite foreign key maps each column to the right target
const POSTGRES_KEYS: &str = "
    select 'PRIMARY KEY', t.relname, a.attname, null, null
    from pg_constraint k
    join pg_class t on t.oid = k.conrelid
    join pg_namespace n on n.oid = t.relnamespace
    join pg_attribute a on a.attrelid = k.conrelid and a.attnum = any(k.conkey)
    where k.contype = 'p' and n.nspname = current_schema()
    union all
    select 'FOREIGN KEY', t.relname, a.attname,
           case when r.relnamespace = t.relnamespace then r.relname
                else rn.nspname || '.' || r.relname end,
           ra.attname
    from pg_constraint k
    join pg_class t on t.oid = k.conrelid
    join pg_namespace n on n.oid = t.relnamespace
    join pg_class r on r.oid = k.confrelid
    join pg_namespace rn on rn.oid = r.relnamespace
    cross join lateral unnest(k.conkey, k.confkey) as pair(own, other)
    join pg_attribute a on a.attrelid = k.conrelid and a.attnum = pair.own
    join pg_attribute ra on ra.attrelid = k.confrelid and ra.attnum = pair.other
    where k.contype = 'f' and n.nspname = current_schema()";

// format_type names the column the way ddl spells it, arrays, enums and
// domains included, and schema-qualifies a type outside the search path
const POSTGRES_COLUMNS: &str = "
    select c.relname, a.attname, format_type(a.atttypid, a.atttypmod),
           case when a.attnotnull then 'NO' else 'YES' end
    from pg_attribute a
    join pg_class c on c.oid = a.attrelid
    join pg_namespace n on n.oid = c.relnamespace
    where n.nspname = current_schema()
      and c.relkind in ('r', 'p', 'v', 'm', 'f')
      and not c.relispartition
      and a.attnum > 0
      and not a.attisdropped
    order by c.relname, a.attnum";

const GREPTIME_COLUMNS: &str = "
    select table_name, column_name, data_type, is_nullable
    from information_schema.columns
    where table_schema = current_schema()
    order by table_name, ordinal_position";

async fn postgres_schema(session: &Session) -> Result<Vec<TableSchema>, String> {
    let greptime = session.flavour == "greptimedb";
    let columns = read(
        session,
        if greptime {
            GREPTIME_COLUMNS
        } else {
            POSTGRES_COLUMNS
        },
    )
    .await?;

    let keys = if greptime {
        QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected: None,
        }
    } else {
        read(session, POSTGRES_KEYS).await?
    };

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

        foreign
            .entry((table, column))
            .or_insert_with(|| format!("{target_table}.{target_column}"));
    }

    let mut grouped: Vec<TableSchema> = Vec::new();

    for row in columns.rows {
        let table = row[0].clone().unwrap_or_default();
        let name = row[1].clone().unwrap_or_default();
        let data_type = row[2].clone().unwrap_or_default();
        let required = row[3].as_deref() == Some("NO");

        if grouped
            .last()
            .map(|last| last.name != table)
            .unwrap_or(true)
        {
            grouped.push(TableSchema {
                name: table.clone(),
                ..Default::default()
            });
        }

        let Some(current) = grouped.last_mut() else {
            continue;
        };

        current.columns.push(ColumnInfo {
            primary_key: primary.contains(&(table.clone(), name.clone())),
            references: foreign.get(&(table.clone(), name.clone())).cloned(),
            name,
            data_type,
            required,
            note: None,
        });
    }

    Ok(grouped)
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

    let words = text.split_whitespace().collect::<Vec<_>>();
    let follows_ref = |index: usize| index > 0 && words[index - 1] == "@ref";

    let hints = words
        .iter()
        .enumerate()
        .filter_map(|(index, word)| match word.strip_prefix("@ref:") {
            Some(target) => Some(target),
            None if follows_ref(index) => Some(*word),
            None => None,
        })
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'))
        .filter(|word| word.contains('.'))
        .map(str::to_string)
        .collect::<Vec<_>>();

    let extra = words
        .iter()
        .enumerate()
        .filter(|(index, word)| !word.starts_with("@ref") && !follows_ref(*index))
        .map(|(_, word)| *word)
        .collect::<Vec<_>>()
        .join(" ");

    (Some(extra.trim().to_string()), hints)
}

async fn postgres_notes(session: &Session) -> Result<Vec<(String, String, String)>, String> {
    let listing = read(
        session,
        "select c.relname, coalesce(a.attname, ''), d.description
         from pg_description d
         join pg_class c on c.oid = d.objoid
         join pg_namespace n on n.oid = c.relnamespace
         left join pg_attribute a
           on a.attrelid = c.oid and a.attnum = d.objsubid
         where n.nspname = current_schema()",
    )
    .await?;

    Ok(listing
        .rows
        .into_iter()
        .map(|row| {
            (
                row[0].clone().unwrap_or_default(),
                row[1].clone().unwrap_or_default(),
                row[2].clone().unwrap_or_default(),
            )
        })
        .collect())
}

fn bare(counts: &HashMap<String, i64>) -> Vec<TableSchema> {
    let mut names: Vec<&String> = counts.keys().collect();
    names.sort();

    names
        .into_iter()
        .map(|name| TableSchema {
            name: name.clone(),
            ..Default::default()
        })
        .collect()
}

fn mysql_schema(listing: QueryResult) -> Vec<TableSchema> {
    let mut grouped: Vec<TableSchema> = Vec::new();

    for row in listing.rows {
        let cell = |index: usize| row.get(index).cloned().flatten().unwrap_or_default();
        let table = cell(0);

        if grouped
            .last()
            .map(|last| last.name != table)
            .unwrap_or(true)
        {
            grouped.push(TableSchema {
                name: table.clone(),
                ..Default::default()
            });
        }

        let target = cell(5);

        let Some(current) = grouped.last_mut() else {
            continue;
        };

        current.columns.push(ColumnInfo {
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

    grouped
}

fn sqlite_schema(
    connection: &Connection,
    mut names: Vec<String>,
) -> Result<Vec<TableSchema>, String> {
    names.sort();

    let mut out = Vec::new();

    for name in names {
        let info = query_sqlite(
            connection,
            &format!("pragma table_info({})", quote_ident(&name)),
        )?;
        let links = query_sqlite(
            connection,
            &format!("pragma foreign_key_list({})", quote_ident(&name)),
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
            name,
            columns,
            ..Default::default()
        });
    }

    Ok(out)
}
