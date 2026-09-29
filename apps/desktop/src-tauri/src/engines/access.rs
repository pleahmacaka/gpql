use std::ops::ControlFlow;

use sqlparser::ast::{
    ContextModifier, Expr, ObjectName, Query, Select, Set, Statement, TransactionAccessMode,
    TransactionMode, UtilityOption, Visit, Visitor,
};
use sqlparser::dialect::{
    ClickHouseDialect, Dialect, DuckDbDialect, GenericDialect, MySqlDialect, PostgreSqlDialect,
    SQLiteDialect, SnowflakeDialect,
};
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token as SqlToken, Tokenizer};

use super::backends;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    Unknown,
}

pub fn access(kind: &str, sql: &str) -> Access {
    let dialect = backends::find(kind)
        .map(|backend| backend.dialect)
        .unwrap_or(kind);

    match dialect {
        "cypher" => cypher(sql),
        "flux" => flux(sql),
        "s3" => verb(
            sql,
            &["ls", "list", "get", "presign"],
            &["mk", "rmb", "put", "rm"],
        ),
        "mqtt" => verb(sql, &[], &["publish"]),
        _ => match parse(kind, sql) {
            Some(statements) => statements.iter().map(classify).fold(Access::Read, worst),
            None => Access::Unknown,
        },
    }
}

pub fn single_read(kind: &str, sql: &str) -> bool {
    match parse(kind, sql) {
        Some(statements) => statements.len() == 1 && classify(&statements[0]) == Access::Read,
        None => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    None,
    Commit,
    Rollback,
    Savepoint,
    Mixed,
}

// tokens rather than the parser, because a script the parser rejects still runs
pub fn ending(kind: &str, sql: &str) -> Ending {
    let Some(statements) = statements(kind, sql, true) else {
        return Ending::None;
    };

    let found: Vec<Ending> = statements.iter().map(|words| control(words)).collect();
    let ends = |each: &Ending| matches!(each, Ending::Commit | Ending::Rollback | Ending::Mixed);

    match found.as_slice() {
        [only @ (Ending::Commit | Ending::Rollback)] => *only,
        _ if found.iter().any(ends) => Ending::Mixed,
        _ if found.contains(&Ending::Savepoint) => Ending::Savepoint,
        _ => Ending::None,
    }
}

fn control(words: &[String]) -> Ending {
    let second = words.get(1).map(String::as_str).unwrap_or_default();
    let plain = words[1..]
        .iter()
        .all(|word| matches!(word.as_str(), "work" | "transaction"));

    match words[0].as_str() {
        "commit" | "end" if plain => Ending::Commit,
        "rollback" | "abort" if plain => Ending::Rollback,
        "rollback" if words.iter().any(|word| word == "to") => Ending::Savepoint,
        "savepoint" | "release" => Ending::Savepoint,
        "begin" | "commit" | "rollback" | "end" | "abort" | "xa" => Ending::Mixed,
        "start" | "prepare" if second == "transaction" => Ending::Mixed,
        "set" if words.iter().any(|word| word.contains("autocommit")) => Ending::Mixed,
        _ => Ending::None,
    }
}

const PLAIN_READS: &[&str] = &[
    "select", "with", "show", "explain", "describe", "desc", "values", "table",
];

// a server that refuses writes still lets a file export or a setting change through
pub fn plain_read(kind: &str, sql: &str) -> bool {
    let Some(statements) = statements(kind, sql, false) else {
        return false;
    };

    let [words] = statements.as_slice() else {
        return false;
    };

    let risky = |word: &String| {
        matches!(word.as_str(), "into" | "analyze" | "analyse")
            || word.contains("read_only")
            || word.contains("readonly")
            || side_effect(word)
    };

    PLAIN_READS.contains(&words[0].as_str()) && !words.iter().any(risky)
}

const KEEPS_TRANSACTION: &[&str] = &[
    "select",
    "with",
    "values",
    "table",
    "insert",
    "update",
    "delete",
    "replace",
    "explain",
    "describe",
    "desc",
    "show",
    "savepoint",
    "release",
    "rollback",
    "do",
    "call",
    "use",
];

// mysql silently commits an open transaction before any statement not listed here
pub fn keeps_transaction(kind: &str, sql: &str) -> bool {
    let Some(statements) = statements(kind, sql, true) else {
        return false;
    };

    statements.iter().all(|words| {
        let second = words.get(1).map(String::as_str).unwrap_or_default();

        match words[0].as_str() {
            "create" | "drop" => second == "temporary",
            "load" => matches!(second, "data" | "xml"),
            "set" => !words.iter().any(|word| word.contains("autocommit")),
            first => KEEPS_TRANSACTION.contains(&first),
        }
    })
}

pub fn calls_routine(kind: &str, sql: &str) -> bool {
    statements(kind, sql, true)
        .is_some_and(|statements| statements.iter().any(|words| words[0] == "call"))
}

// a begin inside a statement opens a trigger or routine body whose semicolons don't split it
fn statements(kind: &str, sql: &str, bodies: bool) -> Option<Vec<Vec<String>>> {
    let tokens = Tokenizer::new(dialect(kind).as_ref(), sql)
        .tokenize()
        .ok()?;

    let mut out: Vec<Vec<String>> = vec![Vec::new()];
    let mut depth = 0usize;
    let mut closing = false;

    for token in tokens {
        let word = match token {
            SqlToken::SemiColon => {
                if closing {
                    depth = depth.saturating_sub(1);
                    closing = false;
                }

                if depth == 0 {
                    out.push(Vec::new());
                }

                continue;
            }
            SqlToken::Word(word) if word.quote_style.is_none() => word.value.to_ascii_lowercase(),
            _ => continue,
        };

        let current = out.len() - 1;

        if bodies {
            let named_end = closing && matches!(word.as_str(), "if" | "loop" | "while" | "repeat");
            let case_end = closing && word == "case";

            if closing && !named_end {
                depth = depth.saturating_sub(1);
            }

            closing = false;

            match word.as_str() {
                _ if named_end || case_end => {}
                "begin" if !out[current].is_empty() => depth += 1,
                "case" => depth += 1,
                "end" => closing = true,
                _ => {}
            }
        }

        out[current].push(word);
    }

    out.retain(|words| !words.is_empty());

    Some(out)
}

fn worst(held: Access, next: Access) -> Access {
    match (held, next) {
        (Access::Write, _) | (_, Access::Write) => Access::Write,
        (Access::Unknown, _) | (_, Access::Unknown) => Access::Unknown,
        _ => Access::Read,
    }
}

fn dialect(kind: &str) -> Box<dyn Dialect> {
    match kind {
        "postgres" | "supabase" | "supabase_api" => Box::new(PostgreSqlDialect {}),
        "mysql" => Box::new(MySqlDialect {}),
        "sqlite" | "turso" | "d1" => Box::new(SQLiteDialect {}),
        "duckdb" => Box::new(DuckDbDialect {}),
        "clickhouse" => Box::new(ClickHouseDialect {}),
        "snowflake" => Box::new(SnowflakeDialect {}),
        _ => Box::new(GenericDialect {}),
    }
}

fn parse(kind: &str, sql: &str) -> Option<Vec<Statement>> {
    Parser::parse_sql(dialect(kind).as_ref(), sql).ok()
}

fn classify(statement: &Statement) -> Access {
    match statement {
        Statement::Query(query) => nested(query.as_ref()),
        Statement::Explain {
            analyze,
            options,
            statement,
            ..
        } => {
            if *analyze || analyzes(options.as_deref()) {
                classify(statement)
            } else {
                Access::Read
            }
        }
        Statement::StartTransaction {
            modes, statements, ..
        } => {
            if read_write(modes) {
                Access::Write
            } else {
                statements.iter().map(classify).fold(Access::Read, worst)
            }
        }
        Statement::Set(set) => setting(set),
        Statement::Pragma { name, value, is_eq } => pragma(name, value.is_some(), *is_eq),
        Statement::ExplainTable { .. }
        | Statement::ShowFunctions { .. }
        | Statement::ShowVariable { .. }
        | Statement::ShowStatus { .. }
        | Statement::ShowVariables { .. }
        | Statement::ShowCreate { .. }
        | Statement::ShowColumns { .. }
        | Statement::ShowCatalogs { .. }
        | Statement::ShowDatabases { .. }
        | Statement::ShowProcessList { .. }
        | Statement::ShowSchemas { .. }
        | Statement::ShowCharset(_)
        | Statement::ShowObjects(_)
        | Statement::ShowTables { .. }
        | Statement::ShowViews { .. }
        | Statement::ShowCollation { .. }
        | Statement::Use(_)
        | Statement::Commit { .. }
        | Statement::Rollback { .. }
        | Statement::Savepoint { .. }
        | Statement::ReleaseSavepoint { .. } => Access::Read,
        _ => Access::Write,
    }
}

fn analyzes(options: Option<&[UtilityOption]>) -> bool {
    options.unwrap_or_default().iter().any(|option| {
        let switched_off = option.arg.as_ref().is_some_and(|arg| {
            matches!(
                arg.to_string().to_ascii_lowercase().as_str(),
                "false" | "off" | "0"
            )
        });

        option.name.value.eq_ignore_ascii_case("analyze") && !switched_off
    })
}

fn read_write(modes: &[TransactionMode]) -> bool {
    modes.iter().any(|mode| {
        matches!(
            mode,
            TransactionMode::AccessMode(TransactionAccessMode::ReadWrite)
        )
    })
}

fn setting(set: &Set) -> Access {
    let harmless = |name: &ObjectName, scope: Option<&ContextModifier>| {
        let text = name.to_string().to_ascii_lowercase();

        !matches!(scope, Some(ContextModifier::Global))
            && !text.contains("read_only")
            && !text.contains("readonly")
            && !text.contains("autocommit")
            && !text.starts_with("@@global")
    };

    let fine = match set {
        Set::SingleAssignment {
            scope, variable, ..
        } => harmless(variable, scope.as_ref()),
        Set::ParenthesizedAssignments { variables, .. } => {
            variables.iter().all(|variable| harmless(variable, None))
        }
        Set::MultipleAssignments { assignments } => assignments
            .iter()
            .all(|assigned| harmless(&assigned.name, assigned.scope.as_ref())),
        Set::SetTransaction { modes, .. } => !read_write(modes),
        Set::SetTimeZone { .. } | Set::SetNames { .. } | Set::SetNamesDefault {} => true,
        Set::SetSessionParam(_) | Set::SetSessionAuthorization(_) | Set::SetRole { .. } => false,
    };

    if fine {
        Access::Read
    } else {
        Access::Write
    }
}

const PRAGMAS_WITH_ARGUMENT: &[&str] = &[
    "table_info",
    "table_xinfo",
    "table_list",
    "index_list",
    "index_info",
    "index_xinfo",
    "foreign_key_list",
    "foreign_key_check",
    "integrity_check",
    "quick_check",
    "storage_info",
    "show",
];

const PRAGMAS_BARE: &[&str] = &[
    "database_list",
    "collation_list",
    "function_list",
    "module_list",
    "pragma_list",
    "compile_options",
    "page_count",
    "page_size",
    "freelist_count",
    "data_version",
    "schema_version",
    "user_version",
    "application_id",
    "encoding",
    "journal_mode",
    "foreign_keys",
    "query_only",
    "cache_size",
    "busy_timeout",
    "synchronous",
    "auto_vacuum",
    "max_page_count",
    "database_size",
    "show_tables",
    "show_tables_expanded",
    "version",
    "platform",
    "functions",
    "collations",
];

fn pragma(name: &ObjectName, valued: bool, assigned: bool) -> Access {
    let last = name
        .0
        .last()
        .and_then(|part| part.as_ident())
        .map(|ident| ident.value.to_ascii_lowercase())
        .unwrap_or_default();

    let read = if assigned {
        false
    } else if valued {
        PRAGMAS_WITH_ARGUMENT.contains(&last.as_str())
    } else {
        PRAGMAS_WITH_ARGUMENT.contains(&last.as_str()) || PRAGMAS_BARE.contains(&last.as_str())
    };

    if read {
        Access::Read
    } else {
        Access::Write
    }
}

const SIDE_EFFECTS: &[&str] = &[
    "set_config",
    "nextval",
    "setval",
    "pg_terminate_backend",
    "pg_cancel_backend",
    "pg_reload_conf",
    "pg_rotate_logfile",
    "pg_switch_wal",
    "pg_promote",
    "pg_create_restore_point",
    "pg_create_physical_replication_slot",
    "pg_create_logical_replication_slot",
    "pg_drop_replication_slot",
    "pg_logical_emit_message",
    "pg_stat_reset",
    "pg_notify",
    "pg_file_write",
    "pg_file_rename",
    "pg_file_unlink",
    "lo_import",
    "lo_export",
    "lo_create",
    "lo_creat",
    "lo_unlink",
    "lo_put",
    "lo_from_bytea",
    "dblink",
    "dblink_exec",
    "dblink_connect",
    "dblink_send_query",
    "load_extension",
];

const SYSTEM_READS: &[&str] = &[
    "system$typeof",
    "system$clustering_information",
    "system$clustering_depth",
];

// snowflake's system$ functions cancel queries and change account state
fn side_effect(called: &str) -> bool {
    SIDE_EFFECTS.contains(&called)
        || (called.starts_with("system$") && !SYSTEM_READS.contains(&called))
}

struct Writes;

impl Visitor for Writes {
    type Break = ();

    fn pre_visit_statement(&mut self, _statement: &Statement) -> ControlFlow<()> {
        ControlFlow::Break(())
    }

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<()> {
        let locking = !query.locks.is_empty();
        let unlocking = query
            .settings
            .iter()
            .flatten()
            .any(|setting| setting.key.value.to_ascii_lowercase().contains("readonly"));

        if locking || unlocking {
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    fn pre_visit_select(&mut self, select: &Select) -> ControlFlow<()> {
        if select.into.is_some() {
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<()> {
        if let Expr::Function(function) = expr {
            let called = function
                .name
                .0
                .last()
                .and_then(|part| part.as_ident())
                .map(|ident| ident.value.to_ascii_lowercase())
                .unwrap_or_default();

            if side_effect(&called) {
                return ControlFlow::Break(());
            }
        }

        ControlFlow::Continue(())
    }
}

fn nested(query: &Query) -> Access {
    match query.visit(&mut Writes) {
        ControlFlow::Break(()) => Access::Write,
        ControlFlow::Continue(()) => Access::Read,
    }
}

fn verb(text: &str, reads: &[&str], writes: &[&str]) -> Access {
    let word = text
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if reads.contains(&word.as_str()) {
        return Access::Read;
    }

    if writes.contains(&word.as_str()) {
        return Access::Write;
    }

    Access::Unknown
}

#[derive(Clone, Copy, PartialEq)]
enum Language {
    Cypher,
    Flux,
}

#[derive(Debug, PartialEq)]
enum Token {
    Word(String),
    Text(String),
    Mark(char),
}

struct Scanner<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    out: Vec<Token>,
}

impl Scanner<'_> {
    fn quoted(&mut self, close: char) -> Option<String> {
        let mut body = String::new();

        loop {
            let next = self.chars.next()?;

            if next == '\\' && close != '`' {
                body.push(self.chars.next()?);
                continue;
            }

            if next == close {
                if close == '`' && self.chars.peek() == Some(&'`') {
                    self.chars.next();
                    body.push('`');
                    continue;
                }

                return Some(body);
            }

            body.push(next);
        }
    }

    fn line_comment(&mut self) {
        for next in self.chars.by_ref() {
            if next == '\n' {
                break;
            }
        }
    }

    fn block_comment(&mut self) -> Option<()> {
        let mut last = ' ';

        loop {
            let next = self.chars.next()?;

            if last == '*' && next == '/' {
                return Some(());
            }

            last = next;
        }
    }

    fn word(&mut self, first: char) -> String {
        let mut word = String::from(first);

        while let Some(&next) = self.chars.peek() {
            if !(next.is_alphanumeric() || next == '_') {
                break;
            }

            word.push(next);
            self.chars.next();
        }

        word
    }

    fn regex(&mut self) -> Option<()> {
        loop {
            match self.chars.next()? {
                '\\' => {
                    self.chars.next()?;
                }
                '/' => return Some(()),
                '\n' => return None,
                _ => {}
            }
        }
    }

    fn expects_operand(&self) -> bool {
        match self.out.last() {
            None => true,
            Some(Token::Mark(mark)) => matches!(mark, '~' | '(' | ',' | ':' | '[' | '=' | '{'),
            Some(_) => false,
        }
    }

    fn scan(text: &str, language: Language) -> Option<Vec<Token>> {
        let cypher = language == Language::Cypher;
        let mut scanner = Scanner {
            chars: text.chars().peekable(),
            out: Vec::new(),
        };

        while let Some(next) = scanner.chars.next() {
            match next {
                '/' if scanner.chars.peek() == Some(&'/') => scanner.line_comment(),
                '/' if !cypher && scanner.expects_operand() => scanner.regex()?,
                '/' if cypher && scanner.chars.peek() == Some(&'*') => {
                    scanner.chars.next();
                    scanner.block_comment()?;
                }
                '"' => {
                    let body = scanner.quoted('"')?;

                    scanner.out.push(Token::Text(body));
                }
                '\'' | '`' if cypher => {
                    let body = scanner.quoted(next)?;

                    scanner.out.push(Token::Text(body));
                }
                _ if next.is_alphabetic() || next == '_' => {
                    let word = scanner.word(next);

                    scanner.out.push(Token::Word(word));
                }
                _ if next.is_whitespace() => {}
                _ => scanner.out.push(Token::Mark(next)),
            }
        }

        Some(scanner.out)
    }
}

fn mark_at(tokens: &[Token], at: Option<usize>) -> Option<char> {
    match tokens.get(at?) {
        Some(Token::Mark(mark)) => Some(*mark),
        _ => None,
    }
}

const CYPHER_WRITES: &[&str] = &[
    "CREATE",
    "MERGE",
    "SET",
    "DELETE",
    "DETACH",
    "REMOVE",
    "DROP",
    "FOREACH",
    "INSERT",
    "ALTER",
    "GRANT",
    "DENY",
    "REVOKE",
    "RENAME",
    "TERMINATE",
];

const CYPHER_READ_PROCEDURES: &[&str] = &[
    "db.labels",
    "db.relationshiptypes",
    "db.propertykeys",
    "db.indexes",
    "db.constraints",
    "db.info",
    "db.ping",
    "dbms.components",
    "dbms.procedures",
    "dbms.functions",
    "dbms.info",
    "db.idx.fulltext.querynodes",
    "db.idx.fulltext.queryrelationships",
    "db.idx.vector.querynodes",
    "db.idx.vector.queryrelationships",
];

fn cypher(text: &str) -> Access {
    let Some(tokens) = Scanner::scan(text, Language::Cypher) else {
        return Access::Unknown;
    };

    for (at, token) in tokens.iter().enumerate() {
        let Token::Word(word) = token else {
            continue;
        };

        let before = mark_at(&tokens, at.checked_sub(1));
        let after = mark_at(&tokens, Some(at + 1));

        if matches!(before, Some('.' | ':' | '$')) || after == Some(':') {
            continue;
        }

        let word = word.to_ascii_uppercase();
        let next_word = match tokens.get(at + 1) {
            Some(Token::Word(next)) => next.to_ascii_uppercase(),
            _ => String::new(),
        };

        if CYPHER_WRITES.contains(&word.as_str())
            || (word == "LOAD" && next_word == "CSV")
            || (matches!(word.as_str(), "START" | "STOP") && next_word == "DATABASE")
        {
            return Access::Write;
        }

        if word == "CALL" && after != Some('{') && !read_procedure(&tokens[at + 1..]) {
            return Access::Write;
        }
    }

    Access::Read
}

fn read_procedure(rest: &[Token]) -> bool {
    let mut name = String::new();

    for token in rest {
        match token {
            Token::Word(part) | Token::Text(part) => name.push_str(&part.to_ascii_lowercase()),
            Token::Mark('.') => name.push('.'),
            Token::Mark(_) => break,
        }
    }

    CYPHER_READ_PROCEDURES.contains(&name.as_str()) || name.starts_with("db.schema.")
}

const FLUX_IMPORTS: &[&str] = &[
    "strings",
    "regexp",
    "math",
    "date",
    "array",
    "dict",
    "json",
    "types",
    "timezone",
    "system",
    "runtime",
    "universe",
    "sampledata",
    "influxdata/influxdb",
    "influxdata/influxdb/schema",
    "influxdata/influxdb/v1",
    "influxdata/influxdb/sample",
    "experimental",
    "experimental/aggregate",
    "experimental/array",
    "experimental/table",
];

fn flux(text: &str) -> Access {
    let Some(tokens) = Scanner::scan(text, Language::Flux) else {
        return Access::Unknown;
    };

    let mut verdict = Access::Read;

    for (at, token) in tokens.iter().enumerate() {
        let Token::Word(word) = token else {
            continue;
        };

        if word == "import" {
            let path = tokens[at + 1..].iter().take(2).find_map(|next| match next {
                Token::Text(path) => Some(path.as_str()),
                _ => None,
            });

            if !path.is_some_and(|path| FLUX_IMPORTS.contains(&path)) {
                verdict = Access::Unknown;
            }

            continue;
        }

        if !matches!(word.as_str(), "to" | "wideTo") {
            continue;
        }

        let member = mark_at(&tokens, at.checked_sub(1)) == Some('.');
        let next = mark_at(&tokens, Some(at + 1));
        let label = next == Some(':');

        if next == Some('(') || !(member || label) {
            return Access::Write;
        }
    }

    verdict
}
