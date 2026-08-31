use serde::Deserialize;

use super::db::{literal, query, quote_for, Engine, QueryResult, Session};

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

    return true;
}

// backslash needs doubling inside a MySQL string literal but not a Postgres
// one, so escape with a character no dialect treats specially
const LIKE_ESCAPE: char = '!';

pub(crate) fn like_pattern(value: &str, lead: bool, trail: bool) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        if matches!(character, LIKE_ESCAPE | '%' | '_') {
            escaped.push(LIKE_ESCAPE);
        }

        escaped.push(character);
    }

    return format!(
        "{}{escaped}{}",
        if lead { "%" } else { "" },
        if trail { "%" } else { "" }
    );
}

fn like(column: &str, value: &str, lead: bool, trail: bool) -> String {
    let pattern = literal(&Some(like_pattern(value, lead, trail)));

    return format!("{column} like {pattern} escape '{LIKE_ESCAPE}'");
}

fn condition(session: &Session, filter: &Filter) -> String {
    return predicate(&quote_for(session, &filter.column), filter);
}

pub(crate) fn predicate(column: &str, filter: &Filter) -> String {
    let value = Some(filter.value.clone());

    return match filter.op {
        Op::IsNull => format!("{column} is null"),
        Op::NotNull => format!("{column} is not null"),
        Op::Eq => format!("{column} = {}", literal(&value)),
        Op::Ne => format!("{column} <> {}", literal(&value)),
        Op::Gt => format!("{column} > {}", literal(&value)),
        Op::Gte => format!("{column} >= {}", literal(&value)),
        Op::Lt => format!("{column} < {}", literal(&value)),
        Op::Lte => format!("{column} <= {}", literal(&value)),
        Op::Contains => like(column, &filter.value, true, true),
        Op::Starts => like(column, &filter.value, false, true),
        Op::Ends => like(column, &filter.value, true, false),
    };
}

pub async fn table_rows(
    session: &Session,
    table: &str,
    slice: &Slice,
) -> Result<QueryResult, String> {
    return query(session, &rows_query(session, table, slice)).await;
}

// the builder shows the user the very query the grid would run, so both go
// through here rather than each writing its own
pub fn rows_query(session: &Session, table: &str, slice: &Slice) -> String {
    return shaped_query(session, table, slice, &Shape::default());
}

pub fn shaped_query(
    session: &Session,
    table: &str,
    slice: &Slice,
    shape: &Shape,
) -> String {
    let limit = slice.limit;

    if let Engine::Driver(driver) = &session.engine {
        if let Some(script) = driver.rows_query(table, slice, shape) {
            return script;
        }
    }

    if let Engine::Graph(_) = &session.engine {
        return format!(
            "match (n:{table}) return n skip {} limit {limit}",
            slice.offset
        );
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

    let mut sql = format!("select {chosen} from {}", quote_for(session, table));

    if !slice.filters.is_empty() {
        let conditions = slice
            .filters
            .iter()
            .map(|filter| condition(session, filter))
            .collect::<Vec<_>>()
            .join(" and ");

        sql.push_str(&format!(" where {conditions}"));
    }

    if let Some(sort) = &slice.sort {
        sql.push_str(&format!(
            " order by {} {}",
            quote_for(session, &sort.column),
            if sort.descending { "desc" } else { "asc" }
        ));
    }

    sql.push_str(&format!(" limit {limit} offset {}", slice.offset));

    return sql;
}
