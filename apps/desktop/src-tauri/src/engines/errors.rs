use tokio_postgres::Error as PgError;

pub fn friendly(error: impl std::fmt::Display) -> String {
    let text = error.to_string();

    return text
        .strip_prefix("error connecting to server: ")
        .unwrap_or(&text)
        .to_string();
}

pub fn friendly_pg(error: PgError) -> String {
    if let Some(reported) = error.as_db_error() {
        return reported.message().to_string();
    }

    let mut text = error.to_string();
    let mut cause = std::error::Error::source(&error);

    while let Some(inner) = cause {
        text = format!("{text}: {inner}");
        cause = inner.source();
    }

    return plain_message(&text);
}

fn plain_message(text: &str) -> String {
    let known = [
        ("password missing", "gpql.needs_password"),
        ("os error 10061", "gpql.no_listener"),
        ("Connection refused", "gpql.no_listener"),
        ("os error 10060", "gpql.no_answer"),
        ("os error 11004", "gpql.ipv6_only"),
        ("ENOIDENTIFIER", "gpql.needs_tenant"),
        ("failed to lookup address", "gpql.bad_host"),
    ];

    for (needle, friendly) in known {
        if text.contains(needle) {
            return friendly.to_string();
        }
    }

    return text
        .strip_prefix("error connecting to server: ")
        .unwrap_or(text)
        .to_string();
}
