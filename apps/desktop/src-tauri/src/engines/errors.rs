use tokio_postgres::Error as PgError;

pub fn friendly(error: impl std::fmt::Display) -> String {
    let text = error.to_string();

    text.strip_prefix("error connecting to server: ")
        .unwrap_or(&text)
        .to_string()
}

pub fn friendly_pg(error: PgError) -> String {
    if let Some(reported) = error.as_db_error() {
        let mut text = reported.message().to_string();

        if let Some(detail) = reported.detail() {
            text.push_str(&format!("\nDETAIL: {detail}"));
        }

        if let Some(hint) = reported.hint() {
            text.push_str(&format!("\nHINT: {hint}"));
        }

        return text;
    }

    let mut text = error.to_string();
    let mut cause = std::error::Error::source(&error);

    while let Some(inner) = cause {
        text = format!("{text}: {inner}");
        cause = inner.source();
    }

    friendly(text)
}
