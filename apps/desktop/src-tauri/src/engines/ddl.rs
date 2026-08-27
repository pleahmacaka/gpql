use super::db::{query, quote_ident, Engine, QueryResult, Session};

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
    let quoted = super::db::literal(&Some(table.to_string()));

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
    let quoted = super::db::literal(&Some(view.to_string()));
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

// a view has columns in pg_attribute just like a table does, so ask what the
// object actually is before rebuilding it
async fn postgres_kind(session: &Session, name: &str) -> Result<String, String> {
    let quoted = super::db::literal(&Some(name.to_string()));
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

pub async fn table_ddl(session: &Session, table: &str) -> Result<String, String> {
    return match &session.engine {
        Engine::Postgres(_) => match postgres_kind(session, table).await?.as_str() {
            "v" | "m" => postgres_view_ddl(session, table).await,
            _ => postgres_ddl(session, table).await,
        },
        Engine::MySql(_) => {
            let result = query(session, &format!("show create table `{table}`")).await?;

            Ok(cell(&result, 0, 1))
        }
        Engine::Sqlite(_) | Engine::Duck(_) => {
            let quoted = super::db::literal(&Some(table.to_string()));
            let result = query(
                session,
                &format!(
                    "select sql from sqlite_master where name = {quoted} and sql is not null"
                ),
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
