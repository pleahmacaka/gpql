use serde::Serialize;

#[derive(Clone, Copy, PartialEq)]
pub enum Transport {
    Driver,
    Postgres,
    MySql,
    Sqlite,
    DuckDb,
    Http,
    Redis,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub placeholder: &'static str,
    pub secret: bool,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Backend {
    pub id: &'static str,
    pub label: &'static str,
    pub dialect: &'static str,
    pub icon: &'static str,
    pub port: &'static str,
    pub fields: &'static [Field],
    // GPQL connects and runs queries, but reads no column list and cannot page
    // or rank server side, so the schema graph and the grid stay thin
    pub wip: bool,
    #[serde(skip)]
    pub transport: Transport,
}

const fn field(
    key: &'static str,
    label: &'static str,
    placeholder: &'static str,
) -> Field {
    return Field {
        key,
        label,
        placeholder,
        secret: false,
    };
}

const fn secret(key: &'static str, label: &'static str) -> Field {
    return Field {
        key,
        label,
        placeholder: "",
        secret: true,
    };
}

const SERVER: &[Field] = &[
    field("host", "Host", "127.0.0.1"),
    field("port", "Port", ""),
    field("user", "User", ""),
    secret("password", "Password"),
    field("database", "Database", ""),
    field("tls", "TLS", ""),
];

const FILE: &[Field] = &[field("path", "File", "C:\\path\\to\\app.db")];

const URL_TOKEN: &[Field] = &[
    field("url", "URL", "https://"),
    secret("token", "Token"),
    field("database", "Database", ""),
];

const SUPABASE: &[Field] = &[
    field("host", "Project ref or host", "abcdefgh"),
    field("user", "User", "postgres"),
    secret("password", "Database password"),
    field("database", "Database", "postgres"),
    field("tls", "TLS", ""),
];

const REDIS_URL: &[Field] = &[
    field("url", "URL", "redis://127.0.0.1:6379"),
    field("database", "Graph", "falkordb"),
];

const NEO4J: &[Field] = &[
    field("url", "URL", "neo4j://127.0.0.1:7687"),
    field("user", "User", "neo4j"),
    secret("password", "Password"),
    field("database", "Database", "neo4j"),
];

const SNOWFLAKE: &[Field] = &[
    field("host", "Account", "org-account"),
    field("user", "User", ""),
    secret("password", "Password"),
    field("warehouse", "Warehouse", ""),
    field("database", "Database", ""),
    field("schema", "Schema", "PUBLIC"),
];

const SUPABASE_API: &[Field] = &[
    secret("token", "Access token"),
    field("database", "Project", "abcdefgh"),
];

const D1: &[Field] = &[
    field("url", "Account ID", ""),
    field("database", "Database ID", ""),
    secret("token", "API token"),
];

const FLUX: &[Field] = &[
    field("url", "URL", "http://127.0.0.1:8086"),
    field("user", "Organization", ""),
    secret("token", "API token"),
    field("database", "Bucket", ""),
];

const INFLUX3: &[Field] = &[
    field("url", "URL", "http://127.0.0.1:8181"),
    secret("token", "Token"),
    field("database", "Database", ""),
];

const CLICKHOUSE: &[Field] = &[
    field("url", "URL", "http://127.0.0.1:8123"),
    field("user", "User", "default"),
    secret("password", "Password"),
    field("database", "Database", "default"),
];

// ordered by how many developers reach for them, so the picker opens on
// what most people came for
pub const CATALOG: &[Backend] = &[
    Backend {
        id: "postgres",
        label: "PostgreSQL",
        dialect: "sql",
        icon: "simple-icons:postgresql",
        port: "5432",
        fields: SERVER,
        wip: false,
        transport: Transport::Postgres,
    },
    Backend {
        id: "mysql",
        label: "MySQL",
        dialect: "sql",
        icon: "simple-icons:mysql",
        port: "3306",
        fields: SERVER,
        wip: false,
        transport: Transport::MySql,
    },
    Backend {
        id: "sqlite",
        label: "SQLite",
        dialect: "sql",
        icon: "simple-icons:sqlite",
        port: "",
        fields: FILE,
        wip: false,
        transport: Transport::Sqlite,
    },
    Backend {
        id: "supabase",
        label: "Supabase",
        dialect: "sql",
        icon: "simple-icons:supabase",
        port: "5432",
        fields: SUPABASE,
        wip: false,
        transport: Transport::Postgres,
    },
    Backend {
        id: "supabase_api",
        label: "Supabase (token)",
        dialect: "sql",
        icon: "simple-icons:supabase",
        port: "",
        fields: SUPABASE_API,
        wip: false,
        transport: Transport::Http,
    },
    Backend {
        id: "snowflake",
        label: "Snowflake",
        dialect: "sql",
        icon: "simple-icons:snowflake",
        port: "",
        fields: SNOWFLAKE,
        wip: true,
        transport: Transport::Driver,
    },
    Backend {
        id: "duckdb",
        label: "DuckDB",
        dialect: "sql",
        icon: "simple-icons:duckdb",
        port: "",
        fields: FILE,
        wip: false,
        transport: Transport::DuckDb,
    },
    Backend {
        id: "clickhouse",
        label: "ClickHouse",
        dialect: "sql",
        icon: "simple-icons:clickhouse",
        port: "8123",
        fields: CLICKHOUSE,
        wip: true,
        transport: Transport::Driver,
    },
    Backend {
        id: "neo4j",
        label: "Neo4j",
        dialect: "cypher",
        icon: "simple-icons:neo4j",
        port: "7687",
        fields: NEO4J,
        wip: true,
        transport: Transport::Driver,
    },
    Backend {
        id: "influxdb",
        label: "InfluxDB 3",
        dialect: "sql",
        icon: "simple-icons:influxdb",
        port: "8181",
        fields: INFLUX3,
        wip: false,
        transport: Transport::Driver,
    },
    Backend {
        id: "influxdb2",
        label: "InfluxDB 2",
        dialect: "flux",
        icon: "simple-icons:influxdb",
        port: "8086",
        fields: FLUX,
        wip: false,
        transport: Transport::Driver,
    },
    Backend {
        id: "turso",
        label: "Turso",
        dialect: "sql",
        icon: "simple-icons:turso",
        port: "",
        fields: URL_TOKEN,
        wip: true,
        transport: Transport::Driver,
    },
    Backend {
        id: "d1",
        label: "Cloudflare D1",
        dialect: "sql",
        icon: "simple-icons:cloudflare",
        port: "",
        fields: D1,
        wip: true,
        transport: Transport::Http,
    },
    Backend {
        id: "greptimedb",
        label: "GreptimeDB",
        dialect: "sql",
        icon: "simple-icons:greptimedb",
        port: "4003",
        fields: SERVER,
        wip: false,
        transport: Transport::Postgres,
    },
    Backend {
        id: "falkordb",
        label: "FalkorDB",
        dialect: "cypher",
        icon: "simple-icons:redis",
        port: "6379",
        fields: REDIS_URL,
        wip: true,
        transport: Transport::Redis,
    },
];

pub fn find(id: &str) -> Option<&'static Backend> {
    return CATALOG.iter().find(|backend| backend.id == id);
}

pub fn dialect_of(id: &str) -> &'static str {
    return find(id).map(|backend| backend.dialect).unwrap_or("sql");
}

pub fn transport_of(id: &str) -> Transport {
    return find(id).map(|backend| backend.transport).unwrap_or(Transport::Postgres);
}
