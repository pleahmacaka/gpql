pub mod login;
pub mod tailnet;
pub mod tunnel;

use serde_json::Value;

pub async fn answer(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    let parsed: Option<Value> = serde_json::from_str(&body).ok();

    if status.is_success() {
        return Ok(parsed.unwrap_or(Value::Null));
    }

    let said = parsed.as_ref().and_then(|value| {
        ["/message", "/error/message", "/error"]
            .iter()
            .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
            .map(str::to_string)
    });

    Err(said.unwrap_or_else(|| {
        if body.trim().is_empty() {
            status.to_string()
        } else {
            format!("{status}: {}", body.trim())
        }
    }))
}
