use serde::Serialize;
use serde_json::Value;

use super::db::{query, reads_only, Engine, QueryResult, Session};

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlanNode {
    pub label: String,
    pub detail: String,
    pub rows: Option<f64>,
    pub cost: Option<f64>,
    pub time: Option<f64>,
    pub children: Vec<PlanNode>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub tree: Option<PlanNode>,
    pub text: String,
}

fn number(node: &Value, key: &str) -> Option<f64> {
    return node.get(key).and_then(Value::as_f64);
}

fn text_of(node: &Value, key: &str) -> Option<String> {
    return node
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.to_string());
}

fn shape_postgres(node: &Value) -> PlanNode {
    let mut label = text_of(node, "Node Type").unwrap_or_else(|| "step".into());

    if let Some(relation) = text_of(node, "Relation Name") {
        label = format!("{label} on {relation}");
    }

    let detail = ["Filter", "Index Cond", "Hash Cond", "Join Filter", "Sort Key"]
        .iter()
        .filter_map(|key| {
            node.get(key).map(|value| match value.as_str() {
                Some(text) => format!("{key}: {text}"),
                None => format!("{key}: {value}"),
            })
        })
        .collect::<Vec<_>>()
        .join("\n");

    let children = node
        .get("Plans")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(shape_postgres).collect())
        .unwrap_or_default();

    return PlanNode {
        label,
        detail,
        rows: number(node, "Actual Rows").or_else(|| number(node, "Plan Rows")),
        cost: number(node, "Total Cost"),
        time: number(node, "Actual Total Time"),
        children,
    };
}

fn flat(result: &QueryResult) -> String {
    return result
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| cell.clone().unwrap_or_default())
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect::<Vec<_>>()
        .join("\n");
}

// sqlite reports a flat list with parent ids rather than nesting, so rebuild
// the shape the viewer expects
fn branch(parent: i64, rows: &[(i64, i64, String)]) -> Vec<PlanNode> {
    return rows
        .iter()
        .filter(|(_, owner, _)| *owner == parent)
        .map(|(id, _, label)| PlanNode {
            label: label.clone(),
            children: branch(*id, rows),
            ..Default::default()
        })
        .collect();
}

fn shape_sqlite(result: &QueryResult) -> Option<PlanNode> {
    let rows: Vec<(i64, i64, String)> = result
        .rows
        .iter()
        .filter_map(|row| {
            let id = row.first()?.as_deref()?.parse().ok()?;
            let parent = row.get(1)?.as_deref()?.parse().ok()?;
            let label = row.get(3)?.clone()?;

            Some((id, parent, label))
        })
        .collect();

    let mut top = branch(0, &rows);

    if top.len() == 1 {
        return top.pop();
    }

    if top.is_empty() {
        return None;
    }

    return Some(PlanNode {
        label: "query plan".into(),
        children: top,
        ..Default::default()
    });
}

pub async fn explain(
    session: &Session,
    sql: &str,
    analyze: bool,
) -> Result<Plan, String> {
    // analyze really runs the statement, so an insert would be applied for the
    // sake of timing it
    if analyze && !reads_only(sql) {
        return Err("only a read can be timed with analyze".into());
    }

    return match &session.engine {
        Engine::Postgres(_) => {
            let mode = if analyze {
                "analyze, buffers, format json"
            } else {
                "format json"
            };
            let result = query(session, &format!("explain ({mode}) {sql}")).await?;
            let raw = result
                .rows
                .first()
                .and_then(|row| row.first())
                .and_then(|cell| cell.clone())
                .unwrap_or_default();

            let parsed: Value =
                serde_json::from_str(&raw).map_err(|error| error.to_string())?;
            let root = parsed
                .as_array()
                .and_then(|items| items.first())
                .and_then(|item| item.get("Plan"));

            Ok(Plan {
                tree: root.map(shape_postgres),
                text: raw,
            })
        }
        Engine::MySql(_) => {
            let result = query(session, &format!("explain format=json {sql}")).await?;

            Ok(Plan {
                tree: None,
                text: flat(&result),
            })
        }
        Engine::Sqlite(_) => {
            let result = query(session, &format!("explain query plan {sql}")).await?;

            Ok(Plan {
                tree: shape_sqlite(&result),
                text: flat(&result),
            })
        }
        Engine::Duck(_) => {
            let result = query(session, &format!("explain {sql}")).await?;

            Ok(Plan {
                tree: None,
                text: flat(&result),
            })
        }
        _ => Err("this engine does not explain queries".into()),
    };
}
