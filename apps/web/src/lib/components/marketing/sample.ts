import type { BackendField, BackendInfo, SchemaTable } from "@gpql/ui"

export type Backend = BackendInfo & { driver: string }

const field = (key: string, label: string, placeholder = ""): BackendField => ({
  key,
  label,
  placeholder,
  secret: false,
})

const secret = (key: string, label: string): BackendField => ({
  key,
  label,
  placeholder: "",
  secret: true,
})

const SERVER = [
  field("host", "Host", "127.0.0.1"),
  field("port", "Port"),
  field("user", "User"),
  secret("password", "Password"),
  field("database", "Database"),
  field("tls", "TLS"),
]

const backend = (
  id: string,
  label: string,
  icon: string,
  driver: string,
  extra: Partial<Backend> = {},
): Backend => ({
  id,
  label,
  icon,
  driver,
  dialect: "sql",
  port: "",
  fields: [],
  wip: false,
  ...extra,
})

export const backends: Backend[] = [
  backend(
    "postgres",
    "PostgreSQL",
    "simple-icons:postgresql",
    "tokio-postgres",
    { port: "5432", fields: SERVER },
  ),
  backend("mysql", "MySQL", "simple-icons:mysql", "mysql_async", {
    port: "3306",
    fields: SERVER,
  }),
  backend("sqlite", "SQLite", "simple-icons:sqlite", "rusqlite", {
    fields: [field("path", "File", "C:\\path\\to\\app.db")],
  }),
  backend("duckdb", "DuckDB", "simple-icons:duckdb", "duckdb", {
    fields: [field("path", "File", "C:\\path\\to\\data.duckdb")],
  }),
  backend("supabase", "Supabase", "simple-icons:supabase", "tokio-postgres", {
    fields: [
      field("host", "Project ref or host", "abcdefgh"),
      field("user", "User", "postgres"),
      secret("password", "Database password"),
      field("database", "Database", "postgres"),
    ],
  }),
  backend(
    "greptimedb",
    "GreptimeDB",
    "simple-icons:greptimedb",
    "tokio-postgres",
    { port: "4003", fields: SERVER },
  ),
  backend(
    "influxdb",
    "InfluxDB 3",
    "simple-icons:influxdb",
    "influxdb3-client",
    {
      port: "8181",
      fields: [
        field("url", "URL", "http://127.0.0.1:8181"),
        secret("token", "Token"),
        field("database", "Database"),
      ],
    },
  ),
  backend("influxdb2", "InfluxDB 2", "simple-icons:influxdb", "influxdb2", {
    dialect: "flux",
    port: "8086",
    fields: [
      field("url", "URL", "http://127.0.0.1:8086"),
      field("user", "Organization"),
      secret("token", "API token"),
      field("database", "Bucket"),
    ],
  }),
  backend("snowflake", "Snowflake", "simple-icons:snowflake", "snowflake-api", {
    wip: true,
    fields: [
      field("host", "Account", "org-account"),
      field("user", "User"),
      secret("password", "Password"),
      field("warehouse", "Warehouse"),
      field("database", "Database"),
      field("schema", "Schema", "PUBLIC"),
    ],
  }),
  backend("clickhouse", "ClickHouse", "simple-icons:clickhouse", "clickhouse", {
    port: "8123",
    wip: true,
    fields: [
      field("url", "URL", "http://127.0.0.1:8123"),
      field("user", "User", "default"),
      secret("password", "Password"),
      field("database", "Database", "default"),
    ],
  }),
  backend("neo4j", "Neo4j", "simple-icons:neo4j", "neo4rs", {
    dialect: "cypher",
    port: "7687",
    wip: true,
    fields: [
      field("url", "URL", "neo4j://127.0.0.1:7687"),
      field("user", "User", "neo4j"),
      secret("password", "Password"),
      field("database", "Database", "neo4j"),
    ],
  }),
  backend("turso", "Turso", "simple-icons:turso", "libsql", {
    wip: true,
    fields: [
      field("url", "URL", "https://"),
      secret("token", "Token"),
      field("database", "Database"),
    ],
  }),
  backend("d1", "Cloudflare D1", "simple-icons:cloudflare", "HTTP API", {
    wip: true,
    fields: [
      field("url", "Account ID"),
      field("database", "Database ID"),
      secret("token", "API token"),
    ],
  }),
  backend("falkordb", "FalkorDB", "simple-icons:redis", "redis", {
    dialect: "cypher",
    port: "6379",
    wip: true,
    fields: [
      field("url", "URL", "redis://127.0.0.1:6379"),
      field("database", "Graph", "falkordb"),
    ],
  }),
  backend("mqtt", "MQTT", "simple-icons:mqtt", "rumqttc", {
    dialect: "mqtt",
    port: "1883",
    wip: true,
    fields: [
      field("host", "Host", "127.0.0.1"),
      field("port", "Port"),
      field("user", "User"),
      secret("password", "Password"),
      field("database", "Topic filter", "#"),
    ],
  }),
  backend("s3", "S3", "simple-icons:amazons3", "rust-s3", {
    dialect: "s3",
    wip: true,
    fields: [
      field("url", "Endpoint", "http://127.0.0.1:9000"),
      field("schema", "Region", "us-east-1"),
      field("user", "Access key"),
      secret("password", "Secret key"),
      field("database", "Bucket"),
    ],
  }),
]

const column = (
  name: string,
  dataType: string,
  references: string | null = null,
  required = true,
) => ({ name, dataType, primaryKey: name === "id", required, references })

export const schema: SchemaTable[] = [
  {
    name: "account",
    rows: 4821,
    columns: [
      column("id", "bigint"),
      column("handle", "text"),
      column("display_name", "text", null, false),
      column("created_at", "timestamptz"),
    ],
  },
  {
    name: "space",
    rows: 37,
    columns: [
      column("id", "bigint"),
      column("name", "text"),
      column("created_at", "timestamptz"),
    ],
  },
  {
    name: "channel",
    rows: 214,
    note: "one row per room inside a space",
    columns: [
      column("id", "bigint"),
      column("space_id", "bigint", "space.id"),
      column("topic", "text"),
      column("kind", "text"),
    ],
  },
  {
    name: "membership",
    rows: 9134,
    columns: [
      column("id", "bigint"),
      column("space_id", "bigint", "space.id"),
      column("account_id", "bigint", "account.id"),
      column("role", "text"),
      column("joined_at", "timestamptz"),
    ],
  },
  {
    name: "message",
    rows: 918432,
    columns: [
      column("id", "bigint"),
      column("channel_id", "bigint", "channel.id"),
      column("author_id", "bigint", "account.id"),
      column("body", "text"),
      column("sent_at", "timestamptz"),
      column("edited_at", "timestamptz", null, false),
    ],
  },
  {
    name: "pin",
    rows: 148,
    columns: [
      column("id", "bigint"),
      column("channel_id", "bigint", "channel.id"),
      column("pinned_by", "bigint", "account.id"),
    ],
  },
]

const HANDLES = [
  "mina",
  "joon",
  "ari",
  "theo",
  "sol",
  "noor",
  "kai",
  "ines",
  "hana",
  "remy",
  "yuki",
  "otto",
]

const NAMES = [
  "Mina Park",
  "Joon Lee",
  "Ari Cohen",
  "Theo Grant",
  "Sol Ortiz",
  "Noor Haddad",
  "Kai Tanaka",
  "Ines Moreau",
  "Hana Kim",
  "Remy Laurent",
  "Yuki Sato",
  "Otto Berg",
]

const SPACES = ["acme", "field-notes", "night-shift", "studio", "homelab"]

const TOPICS = [
  "general",
  "releases",
  "support",
  "design",
  "ops",
  "random",
  "hiring",
  "offtopic",
]

const ROLES = ["member", "member", "member", "moderator", "owner"]

const BODIES = [
  "deploy is green, rolling out to eu next",
  "can someone look at the flaky login test?",
  "moved the standup to 10:30",
  "migration 0042 ran fine on staging",
  "the export button now keeps filters",
  "who owns the billing cron?",
  "pushed a fix for the null avatar bug",
  "lunch?",
  "p95 went from 210 ms to 140 ms after the index",
  "draft of the changelog is in the doc",
  "rotated the api keys, update your .env",
  "anyone else seeing 502s from the cdn?",
]

const hash = (index: number, salt: number) =>
  (Math.imul(index + 1, 2654435761) ^ Math.imul(salt + 1, 40503)) >>> 0

const pick = <T>(list: T[], index: number, salt = 1) =>
  list[hash(index, salt) % list.length]

const stamp = (seconds: number) =>
  new Date(Date.UTC(2026, 0, 1) + seconds * 1000)
    .toISOString()
    .slice(0, 19)
    .replace("T", " ")

const within = (index: number, table: string, salt: number) => {
  const total = schema.find(entry => entry.name === table)?.rows ?? 1

  return String((hash(index, salt) % total) + 1)
}

const ROWS: Record<string, (index: number) => (string | null)[]> = {
  account: index => [
    String(index + 1),
    index < HANDLES.length
      ? HANDLES[index]
      : `${pick(HANDLES, index)}${Math.floor(index / HANDLES.length)}`,
    index % 5 === 3 ? null : pick(NAMES, index),
    stamp(index * 5821 + (hash(index, 11) % 3600)),
  ],
  space: index => [
    String(index + 1),
    index < SPACES.length
      ? SPACES[index]
      : `${pick(SPACES, index)}-${Math.floor(index / SPACES.length)}`,
    stamp(index * 180_000 + (hash(index, 12) % 3600)),
  ],
  channel: index => [
    String(index + 1),
    within(index, "space", 2),
    index < TOPICS.length ? TOPICS[index] : `${pick(TOPICS, index)}-${index}`,
    index % 6 === 0 ? "voice" : "text",
  ],
  membership: index => [
    String(index + 1),
    within(index, "space", 3),
    within(index, "account", 4),
    pick(ROLES, index, 5),
    stamp(index * 2460 + (hash(index, 13) % 600)),
  ],
  message: index => [
    String(index + 1),
    within(index, "channel", 6),
    within(index, "account", 7),
    pick(BODIES, index, 8),
    stamp(index * 25 + (hash(index, 14) % 25)),
    index % 11 === 4 ? stamp(index * 25 + 480) : null,
  ],
  pin: index => [
    String(index + 1),
    within(index, "channel", 9),
    within(index, "account", 10),
  ],
}

export function page(table: string, from: number, count: number) {
  const total = schema.find(entry => entry.name === table)?.rows ?? 0
  const end = Math.min(total, from + count)

  return Array.from({ length: Math.max(0, end - from) }, (_, offset) =>
    ROWS[table](from + offset),
  )
}

export const busiest = {
  prompt: "busiest channels this week",
  sql: `select channel.topic, count(*) as messages
from message
join channel on channel.id = message.channel_id
where message.sent_at > now() - interval '7 days'
group by channel.topic
order by messages desc
limit 8`,
  columns: ["topic", "messages"],
  rows: [
    ["releases", "1842"],
    ["general", "1610"],
    ["support", "1377"],
    ["design", "904"],
    ["ops", "766"],
    ["random", "512"],
    ["hiring", "208"],
    ["offtopic", "131"],
  ],
}

export const saved = [
  "busiest channels",
  "stale memberships",
  "pins per space",
  "edits in the last day",
]

export type Found = {
  kind: string
  host: string
  port: string
  user: string
  database: string
  detail: string
  login: boolean
  answer: string
}

export const found: Found[] = [
  {
    kind: "postgres",
    host: "127.0.0.1",
    port: "5432",
    user: "postgres",
    database: "roomy",
    detail: "pg-main",
    login: true,
    answer:
      "PostgreSQL 17.5 on x86_64-pc-linux-musl, compiled by gcc (Alpine 14.2.0) 14.2.0, 64-bit",
  },
  {
    kind: "mysql",
    host: "127.0.0.1",
    port: "3307",
    user: "root",
    database: "shop",
    detail: "my-db",
    login: true,
    answer: "8.4.2",
  },
  {
    kind: "mqtt",
    host: "127.0.0.1",
    port: "1883",
    user: "",
    database: "",
    detail: "broker",
    login: true,
    answer: "14 topics",
  },
  {
    kind: "postgres",
    host: "nas.tail4c2e1.ts.net",
    port: "5432",
    user: "postgres",
    database: "orders",
    detail: "",
    login: true,
    answer: "PostgreSQL 16.9 (Debian 16.9-1.pgdg120+1) on x86_64-pc-linux-gnu",
  },
  {
    kind: "neo4j",
    host: "127.0.0.1",
    port: "7687",
    user: "",
    database: "",
    detail: "graph",
    login: false,
    answer: "",
  },
]
