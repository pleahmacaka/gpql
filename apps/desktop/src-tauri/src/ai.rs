use serde_json::Value;

use crate::db::TableSchema;
use crate::vault::Provider;

const RULES: &str = "You turn a request into one SQL statement for the given schema. \
Answer with SQL only, no prose, no code fences. Prefer read-only statements.";

pub fn outline(tables: &[TableSchema]) -> String {
    return tables
        .iter()
        .map(|table| {
            let columns = table
                .columns
                .iter()
                .map(|column| {
                    match &column.references {
                        Some(target) => format!("{} {} -> {target}", column.name, column.data_type),
                        None => format!("{} {}", column.name, column.data_type),
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!("{}({columns})", table.name)
        })
        .collect::<Vec<_>>()
        .join("\n");
}

pub async fn write_sql(
    provider: &Provider,
    prompt: &str,
    schema: &str,
) -> Result<String, String> {
    let endpoint = format!("{}/chat/completions", provider.base_url.trim_end_matches('/'));

    let answer: Value = reqwest::Client::new()
        .post(endpoint)
        .bearer_auth(&provider.key)
        .json(&serde_json::json!({
            "model": provider.model,
            "temperature": 0,
            "messages": [
                { "role": "system", "content": format!("{RULES}\n\nSchema:\n{schema}") },
                { "role": "user", "content": prompt },
            ],
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    if let Some(message) = answer.pointer("/error/message").and_then(Value::as_str) {
        return Err(message.to_string());
    }

    let text = answer
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| "the model answered with nothing usable".to_string())?;

    return Ok(strip_fences(text));
}

fn strip_fences(text: &str) -> String {
    let trimmed = text.trim();

    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed.to_string();
    };

    let body = rest.split_once('\n').map(|(_, tail)| tail).unwrap_or(rest);

    return body.trim_end_matches("```").trim().to_string();
}
