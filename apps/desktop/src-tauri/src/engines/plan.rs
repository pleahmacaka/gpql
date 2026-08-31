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

// datafusion answers with one row per plan and keeps the whole plan, already
// indented, in the last cell of that row
fn plans(result: &QueryResult) -> String {
    return result
        .rows
        .iter()
        .filter_map(|row| row.last().cloned().flatten())
        .collect::<Vec<_>>()
        .join("\n");
}

fn shape_datafusion(text: &str) -> Option<PlanNode> {
    let mut stack: Vec<(usize, PlanNode)> = Vec::new();
    let mut roots: Vec<PlanNode> = Vec::new();

    fn close(
        depth: usize,
        stack: &mut Vec<(usize, PlanNode)>,
        roots: &mut Vec<PlanNode>,
    ) {
        while stack.last().map(|(open, _)| *open >= depth).unwrap_or(false) {
            let (_, done) = stack.pop().unwrap();

            match stack.last_mut() {
                Some((_, parent)) => parent.children.push(done),
                None => roots.push(done),
            }
        }
    }

    for line in text.lines() {
        let body = line.trim_start();

        if body.is_empty() {
            continue;
        }

        let depth = line.len() - body.len();
        let (label, detail) = match body.split_once(", metrics=") {
            Some((head, metrics)) => (head.to_string(), metrics.to_string()),
            None => (body.to_string(), String::new()),
        };

        close(depth, &mut stack, &mut roots);

        stack.push((
            depth,
            PlanNode {
                label,
                detail,
                ..Default::default()
            },
        ));
    }

    close(0, &mut stack, &mut roots);

    if roots.len() == 1 {
        return roots.pop();
    }

    if roots.is_empty() {
        return None;
    }

    return Some(PlanNode {
        label: "query plan".into(),
        children: roots,
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
        Engine::Driver(_) if session.kind == "influxdb" => {
            let mode = if analyze { "explain analyze" } else { "explain" };
            let result = query(session, &format!("{mode} {sql}")).await?;
            let text = plans(&result);

            Ok(Plan {
                tree: shape_datafusion(&text),
                text,
            })
        }
        _ => Err("this engine does not explain queries".into()),
    };
}

#[cfg(test)]
mod datafusion {
    use super::*;

    #[test]
    fn nests_by_indentation() {
        let tree = shape_datafusion(
            "ProjectionExec: expr=[a]\n  FilterExec: b > 1, metrics=[output_rows=3]\n    ParquetExec: file_groups={1 group}",
        )
        .unwrap();

        assert_eq!(tree.label, "ProjectionExec: expr=[a]");
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].detail, "[output_rows=3]");
        assert_eq!(tree.children[0].children[0].label, "ParquetExec: file_groups={1 group}");
    }

    // captured from influxdb 3.8.3 answering `explain analyze select house,
    // temp from sensor where temp > 20`
    #[test]
    fn reads_a_real_influx_plan() {
        let answered = concat!(
            "CoalesceBatchesExec: target_batch_size=8192, metrics=[output_rows=1]
",
            "  FilterExec: temp@1 > 20, metrics=[output_rows=1]
",
            "    RepartitionExec: partitioning=RoundRobinBatch(16)
",
            "      ProjectionExec: expr=[house@0 as house]
",
            "        DeduplicateExec: [house@0 ASC,time@2 ASC]
",
            "          RecordBatches",
        );

        let tree = shape_datafusion(answered).unwrap();

        assert_eq!(tree.label, "CoalesceBatchesExec: target_batch_size=8192");
        assert_eq!(tree.detail, "[output_rows=1]");

        let mut walk = &tree;
        let mut depth = 1;

        while let Some(child) = walk.children.first() {
            walk = child;
            depth += 1;
        }

        assert_eq!(depth, 6);
        assert_eq!(walk.label, "RecordBatches");
    }

    #[test]
    fn wraps_several_plans() {
        let tree = shape_datafusion("logical_plan\nphysical_plan").unwrap();

        assert_eq!(tree.label, "query plan");
        assert_eq!(tree.children.len(), 2);
    }
}
