use serde::Serialize;

use super::db::{literal, query, Engine, QueryResult, Session};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbObject {
    pub name: String,
    pub kind: String,
    pub detail: String,
}

fn collect(result: QueryResult, kind: &str) -> Vec<DbObject> {
    return result
        .rows
        .into_iter()
        .map(|row| DbObject {
            name: row.first().cloned().flatten().unwrap_or_default(),
            kind: kind.to_string(),
            detail: row.get(1).cloned().flatten().unwrap_or_default(),
        })
        .collect();
}

async fn postgres_objects(session: &Session) -> Result<Vec<DbObject>, String> {
    let mut out = Vec::new();

    let views = query(
        session,
        "select c.relname, case c.relkind when 'm' then 'materialized' else '' end
         from pg_class c
         join pg_namespace n on n.oid = c.relnamespace
         where c.relkind in ('v', 'm') and n.nspname = current_schema()
         order by c.relname",
    )
    .await?;

    out.extend(collect(views, "view"));

    let indexes = query(
        session,
        "select i.relname, t.relname
         from pg_class i
         join pg_index x on x.indexrelid = i.oid
         join pg_class t on t.oid = x.indrelid
         join pg_namespace n on n.oid = i.relnamespace
         where i.relkind = 'i' and n.nspname = current_schema()
         order by t.relname, i.relname",
    )
    .await?;

    out.extend(collect(indexes, "index"));

    let sequences = query(
        session,
        "select c.relname, ''
         from pg_class c
         join pg_namespace n on n.oid = c.relnamespace
         where c.relkind = 'S' and n.nspname = current_schema()
         order by c.relname",
    )
    .await?;

    out.extend(collect(sequences, "sequence"));

    let routines = query(
        session,
        "select p.proname,
                case p.prokind when 'p' then 'procedure' else 'function' end
         from pg_proc p
         join pg_namespace n on n.oid = p.pronamespace
         where n.nspname = current_schema()
         order by p.proname",
    )
    .await?;

    out.extend(collect(routines, "routine"));

    let triggers = query(
        session,
        "select t.tgname, c.relname
         from pg_trigger t
         join pg_class c on c.oid = t.tgrelid
         join pg_namespace n on n.oid = c.relnamespace
         where not t.tgisinternal and n.nspname = current_schema()
         order by c.relname, t.tgname",
    )
    .await?;

    out.extend(collect(triggers, "trigger"));

    let types = query(
        session,
        "select t.typname, case t.typtype when 'e' then 'enum' else 'composite' end
         from pg_type t
         join pg_namespace n on n.oid = t.typnamespace
         where n.nspname = current_schema()
           and t.typtype in ('e', 'c')
           and not exists (
             select 1 from pg_class c
             where c.oid = t.typrelid and c.relkind <> 'c'
           )
         order by t.typname",
    )
    .await?;

    out.extend(collect(types, "type"));

    return Ok(out);
}

async fn sqlite_objects(session: &Session) -> Result<Vec<DbObject>, String> {
    let mut out = Vec::new();

    for (type_name, kind) in [("view", "view"), ("index", "index"), ("trigger", "trigger")] {
        let listed = query(
            session,
            &format!(
                "select name, coalesce(tbl_name, '') from sqlite_master
                 where type = {} and name not like 'sqlite_%'
                 order by name",
                literal(&Some(type_name.to_string()))
            ),
        )
        .await?;

        out.extend(collect(listed, kind));
    }

    return Ok(out);
}

async fn mysql_objects(session: &Session) -> Result<Vec<DbObject>, String> {
    let mut out = Vec::new();

    let views = query(
        session,
        "select table_name, '' from information_schema.views
         where table_schema = database() order by table_name",
    )
    .await?;

    out.extend(collect(views, "view"));

    let routines = query(
        session,
        "select routine_name, lower(routine_type)
         from information_schema.routines
         where routine_schema = database() order by routine_name",
    )
    .await?;

    out.extend(collect(routines, "routine"));

    let triggers = query(
        session,
        "select trigger_name, event_object_table
         from information_schema.triggers
         where trigger_schema = database() order by trigger_name",
    )
    .await?;

    out.extend(collect(triggers, "trigger"));

    let indexes = query(
        session,
        "select distinct index_name, table_name
         from information_schema.statistics
         where table_schema = database() order by table_name, index_name",
    )
    .await?;

    out.extend(collect(indexes, "index"));

    return Ok(out);
}

pub async fn objects(session: &Session) -> Result<Vec<DbObject>, String> {
    return match &session.engine {
        Engine::Postgres(_) => postgres_objects(session).await,
        Engine::Sqlite(_) => sqlite_objects(session).await,
        Engine::MySql(_) => mysql_objects(session).await,
        // the rest expose no catalog beyond their tables
        _ => Ok(Vec::new()),
    };
}
