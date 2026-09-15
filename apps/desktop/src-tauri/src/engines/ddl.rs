use super::db::{literal, query, quote_ident, Engine, QueryResult, Session};

fn cell(result: &QueryResult, row: usize, column: usize) -> String {
    return result
        .rows
        .get(row)
        .and_then(|entry| entry.get(column))
        .and_then(|value| value.clone())
        .unwrap_or_default();
}

// postgres has no SHOW CREATE TABLE, but it will hand back exact definitions
// for types, defaults, constraints and indexes, so rebuild from those instead
// of guessing at the formatting
async fn postgres_ddl(session: &Session, table: &str) -> Result<String, String> {
    let name = quote_ident(table);
    let quoted = literal(&Some(table.to_string()));

    let columns = query(
        session,
        &format!(
            "select a.attname,
                    format_type(a.atttypid, a.atttypmod),
                    a.attnotnull,
                    pg_get_expr(d.adbin, d.adrelid)
             from pg_attribute a
             left join pg_attrdef d on d.adrelid = a.attrelid and d.adnum = a.attnum
             where a.attrelid = to_regclass({quoted})::oid
               and a.attnum > 0
               and not a.attisdropped
             order by a.attnum"
        ),
    )
    .await?;

    if columns.rows.is_empty() {
        return Err("no such table".into());
    }

    let mut lines = Vec::new();

    for row in 0..columns.rows.len() {
        let mut line = format!(
            "  {} {}",
            quote_ident(&cell(&columns, row, 0)),
            cell(&columns, row, 1)
        );

        let default = cell(&columns, row, 3);

        if !default.is_empty() {
            line.push_str(&format!(" default {default}"));
        }

        if cell(&columns, row, 2) == "t" {
            line.push_str(" not null");
        }

        lines.push(line);
    }

    let constraints = query(
        session,
        &format!(
            "select pg_get_constraintdef(oid), conname
             from pg_constraint
             where conrelid = to_regclass({quoted})::oid
             order by contype desc, conname"
        ),
    )
    .await?;

    for row in 0..constraints.rows.len() {
        lines.push(format!(
            "  constraint {} {}",
            quote_ident(&cell(&constraints, row, 1)),
            cell(&constraints, row, 0)
        ));
    }

    let mut out = format!("create table {name} (\n{}\n);", lines.join(",\n"));

    let indexes = query(
        session,
        &format!(
            "select pg_get_indexdef(i.indexrelid)
             from pg_index i
             where i.indrelid = to_regclass({quoted})::oid
               and not i.indisprimary
               and not exists (
                 select 1 from pg_constraint c where c.conindid = i.indexrelid
               )"
        ),
    )
    .await?;

    for row in 0..indexes.rows.len() {
        out.push_str(&format!("\n\n{};", cell(&indexes, row, 0)));
    }

    return Ok(out);
}

async fn postgres_view_ddl(session: &Session, view: &str) -> Result<String, String> {
    let quoted = literal(&Some(view.to_string()));
    let body = query(
        session,
        &format!("select pg_get_viewdef(to_regclass({quoted})::oid, true)"),
    )
    .await?;

    let text = cell(&body, 0, 0);

    if text.is_empty() {
        return Err("no such view".into());
    }

    return Ok(format!(
        "create view {} as\n{}",
        quote_ident(view),
        text.trim()
    ));
}

async fn postgres_index_ddl(session: &Session, name: &str) -> Result<String, String> {
    let quoted = literal(&Some(name.to_string()));
    let result = query(
        session,
        &format!("select pg_get_indexdef({quoted}::regclass)"),
    )
    .await?;

    let text = cell(&result, 0, 0);

    if text.is_empty() {
        return Err("no such index".into());
    }

    return Ok(format!("{text};"));
}

async fn postgres_sequence_ddl(session: &Session, name: &str) -> Result<String, String> {
    let quoted = literal(&Some(name.to_string()));
    let result = query(
        session,
        &format!(
            "select s.seqstart, s.seqincrement, s.seqmin, s.seqmax, s.seqcache,
                    s.seqcycle
             from pg_sequence s
             join pg_class c on c.oid = s.seqrelid
             join pg_namespace n on n.oid = c.relnamespace
             where c.relname = {quoted} and n.nspname = current_schema()"
        ),
    )
    .await?;

    if result.rows.is_empty() {
        return Err("no such sequence".into());
    }

    let cycle = if cell(&result, 0, 5) == "t" {
        " cycle"
    } else {
        ""
    };

    return Ok(format!(
        "create sequence {} start with {} increment by {} minvalue {} maxvalue \
         {} cache {}{};",
        quote_ident(name),
        cell(&result, 0, 0),
        cell(&result, 0, 1),
        cell(&result, 0, 2),
        cell(&result, 0, 3),
        cell(&result, 0, 4),
        cycle
    ));
}

async fn postgres_trigger_ddl(
    session: &Session,
    name: &str,
    detail: Option<&str>,
) -> Result<String, String> {
    let quoted = literal(&Some(name.to_string()));
    let table = match detail {
        Some(parent) if !parent.is_empty() => {
            format!(" and c.relname = {}", literal(&Some(parent.to_string())))
        }
        _ => String::new(),
    };

    let result = query(
        session,
        &format!(
            "select pg_get_triggerdef(t.oid)
             from pg_trigger t
             join pg_class c on c.oid = t.tgrelid
             join pg_namespace n on n.oid = c.relnamespace
             where t.tgname = {quoted} and n.nspname = current_schema(){table}
             limit 1"
        ),
    )
    .await?;

    let text = cell(&result, 0, 0);

    if text.is_empty() {
        return Err("no such trigger".into());
    }

    return Ok(format!("{text};"));
}

async fn postgres_routine_ddl(session: &Session, name: &str) -> Result<String, String> {
    let quoted = literal(&Some(name.to_string()));
    let result = query(
        session,
        &format!(
            "select pg_get_functiondef(p.oid)
             from pg_proc p
             join pg_namespace n on n.oid = p.pronamespace
             where p.proname = {quoted} and n.nspname = current_schema()
             limit 1"
        ),
    )
    .await?;

    let text = cell(&result, 0, 0);

    if text.is_empty() {
        return Err("no such routine".into());
    }

    return Ok(text);
}

async fn postgres_type_ddl(session: &Session, name: &str) -> Result<String, String> {
    let quoted = literal(&Some(name.to_string()));
    let found = query(
        session,
        &format!(
            "select t.typtype::text, t.oid
             from pg_type t
             join pg_namespace n on n.oid = t.typnamespace
             where t.typname = {quoted} and n.nspname = current_schema()"
        ),
    )
    .await?;

    let typtype = cell(&found, 0, 0);
    let oid = cell(&found, 0, 1);

    if typtype == "e" {
        let labels = query(
            session,
            &format!(
                "select enumlabel from pg_enum
                 where enumtypid = {oid}
                 order by enumsortorder"
            ),
        )
        .await?;

        let values: Vec<String> = (0..labels.rows.len())
            .map(|row| literal(&Some(cell(&labels, row, 0))))
            .collect();

        return Ok(format!(
            "create type {} as enum ({});",
            quote_ident(name),
            values.join(", ")
        ));
    }

    if typtype == "c" {
        let attributes = query(
            session,
            &format!(
                "select a.attname, format_type(a.atttypid, a.atttypmod)
                 from pg_type t
                 join pg_attribute a on a.attrelid = t.typrelid
                 where t.oid = {oid} and a.attnum > 0 and not a.attisdropped
                 order by a.attnum"
            ),
        )
        .await?;

        if attributes.rows.is_empty() {
            return Err("no definition recorded for that type".into());
        }

        let fields: Vec<String> = (0..attributes.rows.len())
            .map(|row| {
                format!(
                    "  {} {}",
                    quote_ident(&cell(&attributes, row, 0)),
                    cell(&attributes, row, 1)
                )
            })
            .collect();

        return Ok(format!(
            "create type {} as (\n{}\n);",
            quote_ident(name),
            fields.join(",\n")
        ));
    }

    if typtype.is_empty() {
        return Err("no such type".into());
    }

    return Err("that kind of type has no definition".into());
}

// a view has columns in pg_attribute just like a table does, so ask what the
// object actually is before rebuilding it
async fn postgres_kind(session: &Session, name: &str) -> Result<String, String> {
    let quoted = literal(&Some(name.to_string()));
    let result = query(
        session,
        &format!("select relkind::text from pg_class where oid = to_regclass({quoted})::oid"),
    )
    .await?;

    let kind = cell(&result, 0, 0);

    if kind.is_empty() {
        return Err("no such object".into());
    }

    return Ok(kind);
}

async fn postgres_object_ddl(
    session: &Session,
    name: &str,
    kind: Option<&str>,
    detail: Option<&str>,
) -> Result<String, String> {
    return match kind {
        Some("view") => postgres_view_ddl(session, name).await,
        Some("index") => postgres_index_ddl(session, name).await,
        Some("sequence") => postgres_sequence_ddl(session, name).await,
        Some("trigger") => postgres_trigger_ddl(session, name, detail).await,
        Some("routine") => postgres_routine_ddl(session, name).await,
        Some("type") => postgres_type_ddl(session, name).await,
        _ => match postgres_kind(session, name).await?.as_str() {
            "v" | "m" => postgres_view_ddl(session, name).await,
            _ => postgres_ddl(session, name).await,
        },
    };
}

async fn mysql_index_parent(session: &Session, name: &str) -> Result<String, String> {
    let quoted = literal(&Some(name.to_string()));
    let result = query(
        session,
        &format!(
            "select table_name from information_schema.statistics
             where table_schema = database() and index_name = {quoted}
             limit 1"
        ),
    )
    .await?;

    let table = cell(&result, 0, 0);

    if table.is_empty() {
        return Err("no such index".into());
    }

    return Ok(table);
}

async fn mysql_routine_ddl(
    session: &Session,
    name: &str,
    detail: Option<&str>,
) -> Result<String, String> {
    return match detail {
        Some(verb @ ("procedure" | "function")) => {
            let result = query(session, &format!("show create {verb} `{name}`")).await?;

            Ok(cell(&result, 0, 2))
        }
        _ => match query(session, &format!("show create procedure `{name}`")).await {
            Ok(result) => Ok(cell(&result, 0, 2)),
            Err(_) => {
                let result = query(session, &format!("show create function `{name}`")).await?;

                Ok(cell(&result, 0, 2))
            }
        },
    };
}

async fn mysql_object_ddl(
    session: &Session,
    name: &str,
    kind: Option<&str>,
    detail: Option<&str>,
) -> Result<String, String> {
    return match kind {
        Some("view") => {
            let result = query(session, &format!("show create view `{name}`")).await?;

            Ok(cell(&result, 0, 1))
        }
        Some("trigger") => {
            let result = query(session, &format!("show create trigger `{name}`")).await?;

            Ok(cell(&result, 0, 2))
        }
        Some("routine") => mysql_routine_ddl(session, name, detail).await,
        Some("index") => {
            let parent = match detail {
                Some(table) if !table.is_empty() => table.to_string(),
                _ => mysql_index_parent(session, name).await?,
            };
            let result = query(session, &format!("show create table `{parent}`")).await?;

            Ok(cell(&result, 0, 1))
        }
        _ => {
            let result = query(session, &format!("show create table `{name}`")).await?;

            Ok(cell(&result, 0, 1))
        }
    };
}

pub async fn object_ddl(
    session: &Session,
    name: &str,
    kind: Option<&str>,
    detail: Option<&str>,
) -> Result<String, String> {
    return match &session.engine {
        Engine::Postgres(_) => postgres_object_ddl(session, name, kind, detail).await,
        Engine::MySql(_) => mysql_object_ddl(session, name, kind, detail).await,
        Engine::Sqlite(_) | Engine::Duck(_) => {
            let quoted = literal(&Some(name.to_string()));
            let result = query(
                session,
                &format!("select sql from sqlite_master where name = {quoted} and sql is not null"),
            )
            .await?;

            let text = cell(&result, 0, 0);

            if text.is_empty() {
                return Err("no definition recorded for that object".into());
            }

            Ok(format!("{text};"))
        }
        _ => Err("this engine does not report definitions".into()),
    };
}
