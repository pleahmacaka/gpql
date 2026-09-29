use serde::Deserialize;

use super::backends::dialect_of;
use super::db::{held, literal, query, quote_for, quote_ident, read, Engine, QueryResult, Session};

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Contains,
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Starts,
    Ends,
    IsNull,
    NotNull,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub column: String,
    pub op: Op,
    #[serde(default)]
    pub value: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sort {
    pub column: String,
    pub descending: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Slice {
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub sort: Option<Sort>,
    #[serde(default)]
    pub filters: Vec<Filter>,
    #[serde(default)]
    pub columns: Vec<String>,
}

// the influx builder offers a time range and a rollup that no sql engine has,
// so they ride beside the slice instead of inside it
#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    #[serde(default)]
    pub range: String,
    #[serde(default)]
    pub every: String,
    #[serde(default)]
    pub func: String,
}

// the caller sees only the rows it asked for, so any sort or filter it cannot
// push down here would silently rank a single page instead of the whole table
pub fn sliceable(session: &Session) -> bool {
    if matches!(session.engine, Engine::Graph(_)) {
        return false;
    }

    if let Engine::Driver(driver) = &session.engine {
        return driver.sliceable();
    }

    true
}

// backslash needs doubling inside a MySQL string literal but not a Postgres
// one, so escape with a character no dialect treats specially
const LIKE_ESCAPE: char = '!';

fn escaped_like(value: &str, lead: bool, trail: bool, escape: char) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        if matches!(character, '%' | '_') || character == escape {
            escaped.push(escape);
        }

        escaped.push(character);
    }

    format!(
        "{}{escaped}{}",
        if lead { "%" } else { "" },
        if trail { "%" } else { "" }
    )
}

pub(crate) fn like_pattern(value: &str, lead: bool, trail: bool) -> String {
    escaped_like(value, lead, trail, LIKE_ESCAPE)
}

fn like(flavour: &str, column: &str, value: &str, lead: bool, trail: bool) -> String {
    // clickhouse has no ESCAPE clause and always escapes with a backslash
    if flavour == "clickhouse" {
        let pattern = literal(flavour, &Some(escaped_like(value, lead, trail, '\\')));

        return format!("{column} like {pattern}");
    }

    let pattern = literal(flavour, &Some(like_pattern(value, lead, trail)));

    format!("{column} like {pattern} escape '{LIKE_ESCAPE}'")
}

fn condition(session: &Session, filter: &Filter) -> String {
    predicate(
        &session.flavour,
        &quote_for(session, &filter.column),
        filter,
    )
}

pub(crate) fn predicate(flavour: &str, column: &str, filter: &Filter) -> String {
    let value = literal(flavour, &Some(filter.value.clone()));

    match filter.op {
        Op::IsNull => format!("{column} is null"),
        Op::NotNull => format!("{column} is not null"),
        Op::Eq => format!("{column} = {value}"),
        Op::Ne => format!("{column} <> {value}"),
        Op::Gt => format!("{column} > {value}"),
        Op::Gte => format!("{column} >= {value}"),
        Op::Lt => format!("{column} < {value}"),
        Op::Lte => format!("{column} <= {value}"),
        Op::Contains => like(flavour, column, &filter.value, true, true),
        Op::Starts => like(flavour, column, &filter.value, false, true),
        Op::Ends => like(flavour, column, &filter.value, true, false),
    }
}

pub(crate) fn cypher_name(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

fn cypher_text(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

// a property may hold a number or a string, so equality and search compare the
// text, and ordering compares numbers only when the value is one
fn cypher_condition(filter: &Filter) -> String {
    let property = format!("n.{}", cypher_name(&filter.column));
    let shown = format!("toString({property})");
    let text = cypher_text(&filter.value);
    let number = filter
        .value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite());

    let ordered = |op: &str| match number {
        Some(number) => format!("{property} {op} {number}"),
        None => format!("{shown} {op} {text}"),
    };

    match filter.op {
        Op::IsNull => format!("{property} is null"),
        Op::NotNull => format!("{property} is not null"),
        Op::Eq => format!("{shown} = {text}"),
        Op::Ne => format!("{shown} <> {text}"),
        Op::Gt => ordered(">"),
        Op::Gte => ordered(">="),
        Op::Lt => ordered("<"),
        Op::Lte => ordered("<="),
        Op::Contains => format!("{shown} contains {text}"),
        Op::Starts => format!("{shown} starts with {text}"),
        Op::Ends => format!("{shown} ends with {text}"),
    }
}

pub(crate) fn cypher_rows(label: &str, slice: &Slice) -> String {
    let mut script = format!("match (n:{})", cypher_name(label));

    if !slice.filters.is_empty() {
        let conditions = slice
            .filters
            .iter()
            .map(cypher_condition)
            .collect::<Vec<_>>()
            .join(" and ");

        script.push_str(&format!(" where {conditions}"));
    }

    script.push_str(" return n order by ");

    if let Some(sort) = &slice.sort {
        script.push_str(&format!(
            "n.{} {}, ",
            cypher_name(&sort.column),
            if sort.descending { "desc" } else { "asc" }
        ));
    }

    script.push_str(&format!(
        "id(n) skip {} limit {}",
        slice.offset, slice.limit
    ));

    script
}

pub async fn table_rows(
    session: &Session,
    table: &str,
    slice: &Slice,
) -> Result<QueryResult, String> {
    if let Engine::Driver(driver) = &session.engine {
        if let Some(page) = driver.page(table, slice).await {
            return page;
        }
    }

    query(session, &rows_query(session, table, slice).await).await
}

fn first_column(result: QueryResult) -> Vec<String> {
    result
        .rows
        .into_iter()
        .filter_map(|row| row.into_iter().next().flatten())
        .collect()
}

async fn sqlite_keys(session: &Session, table: &str) -> Result<Vec<String>, String> {
    let name = literal("sqlite", &Some(table.to_string()));
    let keys = first_column(
        read(
            session,
            &format!("select name from pragma_table_info({name}) where pk > 0 order by pk"),
        )
        .await?,
    );

    if !keys.is_empty() {
        return Ok(keys.iter().map(|key| quote_for(session, key)).collect());
    }

    let rowid = read(
        session,
        &format!(
            "select 1 from sqlite_master where type = 'table' and name = {name}
             and upper(sql) not like '%WITHOUT ROWID%'"
        ),
    )
    .await?;

    Ok(if rowid.rows.is_empty() {
        Vec::new()
    } else {
        vec!["rowid".to_string()]
    })
}

async fn primary_keys(session: &Session, table: &str) -> Result<Vec<String>, String> {
    let quoted = |names: Vec<String>| {
        names
            .iter()
            .map(|name| quote_for(session, name))
            .collect::<Vec<_>>()
    };

    let postgres = format!(
        "select a.attname from pg_index i
         join pg_attribute a on a.attrelid = i.indrelid and a.attnum = any(i.indkey)
         where i.indrelid = to_regclass({}) and i.indisprimary
         order by array_position(i.indkey::int2[], a.attnum)",
        literal("postgres", &Some(quote_ident(table)))
    );

    match &session.engine {
        Engine::Postgres(_) if session.flavour == "greptimedb" => Ok(Vec::new()),
        Engine::Postgres(_) => Ok(quoted(first_column(read(session, &postgres).await?))),
        Engine::Http(remote) if remote.flavour == "supabase_api" => {
            Ok(quoted(first_column(read(session, &postgres).await?)))
        }
        Engine::MySql(_) => Ok(quoted(first_column(
            read(
                session,
                &format!(
                    "select column_name from information_schema.key_column_usage
                     where table_schema = database() and table_name = {}
                       and constraint_name = 'PRIMARY'
                     order by ordinal_position",
                    literal("mysql", &Some(table.to_string()))
                ),
            )
            .await?,
        ))),
        Engine::Sqlite(_) | Engine::Http(_) => sqlite_keys(session, table).await,
        Engine::Duck(_) => Ok(quoted(first_column(
            read(
                session,
                &format!(
                    "select unnest(constraint_column_names) from duckdb_constraints()
                     where constraint_type = 'PRIMARY KEY'
                       and schema_name = current_schema() and table_name = {}",
                    literal("duckdb", &Some(table.to_string()))
                ),
            )
            .await?,
        ))),
        Engine::Driver(_) if session.flavour == "turso" => sqlite_keys(session, table).await,
        Engine::Driver(_) if session.flavour == "influxdb" => Ok(vec![quote_ident("time")]),
        Engine::Driver(_) if session.flavour == "clickhouse" => Ok(first_column(
            read(
                session,
                &format!(
                    "select sorting_key from system.tables
                     where database = currentDatabase() and name = {}",
                    literal("clickhouse", &Some(table.to_string()))
                ),
            )
            .await?,
        )
        .into_iter()
        .filter(|key| !key.trim().is_empty())
        .collect()),
        _ => Ok(Vec::new()),
    }
}

pub(crate) async fn binary_columns(
    session: &Session,
    table: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let sql = match &session.engine {
        Engine::MySql(_) => format!(
            "select column_name from information_schema.columns
             where table_schema = database() and table_name = {}
               and data_type in ('binary', 'varbinary', 'tinyblob', 'blob',
                                 'mediumblob', 'longblob')",
            literal("mysql", &Some(table.to_string()))
        ),
        Engine::Duck(_) => format!(
            "select column_name from duckdb_columns()
             where schema_name = current_schema() and table_name = {}
               and data_type = 'BLOB'",
            literal("duckdb", &Some(table.to_string()))
        ),
        Engine::Sqlite(_) | Engine::Http(_) | Engine::Driver(_)
            if matches!(session.flavour.as_str(), "sqlite" | "d1" | "turso") =>
        {
            format!(
                "select name from pragma_table_info({}) where upper(type) like '%BLOB%'",
                literal("sqlite", &Some(table.to_string()))
            )
        }
        _ => return Ok(Default::default()),
    };

    Ok(first_column(read(session, &sql).await?)
        .into_iter()
        .collect())
}

// offset paging only walks a table cleanly when the order is total, so the
// key goes after whatever the user sorted by
async fn tiebreak(session: &Session, table: &str) -> Vec<String> {
    if let Some(known) = held(&session.order_keys).get(table) {
        return known.clone();
    }

    let keys = primary_keys(session, table).await.unwrap_or_default();

    held(&session.order_keys).insert(table.to_string(), keys.clone());

    keys
}

// the builder shows the user the very query the grid would run, so both go
// through here rather than each writing its own
pub async fn rows_query(session: &Session, table: &str, slice: &Slice) -> String {
    shaped_query(session, table, slice, &Shape::default()).await
}

pub async fn shaped_query(session: &Session, table: &str, slice: &Slice, shape: &Shape) -> String {
    if dialect_of(&session.flavour) == "cypher" {
        return cypher_rows(table, slice);
    }

    if let Engine::Driver(driver) = &session.engine {
        if let Some(script) = driver.rows_query(table, slice, shape).await {
            return script;
        }
    }

    let chosen = if slice.columns.is_empty() {
        "*".to_string()
    } else {
        slice
            .columns
            .iter()
            .map(|column| quote_for(session, column))
            .collect::<Vec<_>>()
            .join(", ")
    };

    // a read-only supabase api call may run under a role without our search
    // path, and every table GPQL lists there lives in public
    let source = if session.flavour == "supabase_api" {
        format!("{}.{}", quote_ident("public"), quote_ident(table))
    } else {
        quote_for(session, table)
    };

    let mut sql = format!("select {chosen} from {source}");

    if !slice.filters.is_empty() {
        let conditions = slice
            .filters
            .iter()
            .map(|filter| condition(session, filter))
            .collect::<Vec<_>>()
            .join(" and ");

        sql.push_str(&format!(" where {conditions}"));
    }

    let sorted = slice
        .sort
        .as_ref()
        .map(|sort| quote_for(session, &sort.column));
    let mut order: Vec<String> = slice
        .sort
        .iter()
        .map(|sort| {
            format!(
                "{} {}",
                quote_for(session, &sort.column),
                if sort.descending { "desc" } else { "asc" }
            )
        })
        .collect();

    for key in tiebreak(session, table).await {
        if sorted.as_ref() != Some(&key) {
            order.push(key);
        }
    }

    if !order.is_empty() {
        sql.push_str(&format!(" order by {}", order.join(", ")));
    }

    sql.push_str(&format!(" limit {} offset {}", slice.limit, slice.offset));

    sql
}
