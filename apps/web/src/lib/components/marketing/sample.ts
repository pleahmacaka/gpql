import type { SchemaTable, SessionDraft } from "@gpql/ui"

export const draft: SessionDraft = {
  kind: "postgres",
  host: "127.0.0.1",
  port: "5432",
  user: "postgres",
  password: "hunter2hunter2",
  database: "roomy",
  path: "",
  url: "",
  token: "",
}

export const backends = [
  {
    id: "postgres",
    label: "PostgreSQL",
    dialect: "sql",
    icon: "lucide:database",
    port: "5432",
    fields: [
      { key: "host", label: "Host", placeholder: "127.0.0.1", secret: false },
      { key: "port", label: "Port", placeholder: "", secret: false },
      { key: "user", label: "User", placeholder: "", secret: false },
      { key: "password", label: "Password", placeholder: "", secret: true },
      { key: "database", label: "Database", placeholder: "", secret: false },
    ],
  },
  {
    id: "mysql",
    label: "MySQL",
    dialect: "sql",
    icon: "lucide:database",
    port: "3306",
    fields: [],
  },
  {
    id: "clickhouse",
    label: "ClickHouse",
    dialect: "sql",
    icon: "lucide:bar-chart-3",
    port: "8123",
    fields: [],
  },
  {
    id: "neo4j",
    label: "Neo4j",
    dialect: "cypher",
    icon: "lucide:git-fork",
    port: "7474",
    fields: [],
  },
]

export const columns = [
  "id",
  "channel",
  "sent_at",
  "words",
  "reactions",
  "pinned",
]

const channels = ["general", "releases", "random"]

export const rows: (string | null)[][] = Array.from(
  { length: 240 },
  (_, index) => [
    String(index + 1),
    channels[index % channels.length],
    `2026-08-${String((index % 28) + 1).padStart(2, "0")}`,
    String(6 + ((index * 7) % 48)),
    String((index * 3) % 12),
    index % 9 === 0 ? null : "no",
  ],
)

export const tables: SchemaTable[] = [
  {
    name: "account",
    rows: 4821,
    columns: [
      {
        name: "id",
        dataType: "uuid",
        primaryKey: true,
        required: true,
        references: null,
      },
      {
        name: "handle",
        dataType: "text",
        primaryKey: false,
        required: true,
        references: null,
      },
    ],
  },
  {
    name: "space",
    rows: 37,
    columns: [
      {
        name: "id",
        dataType: "uuid",
        primaryKey: true,
        required: true,
        references: null,
      },
      {
        name: "owner_id",
        dataType: "uuid",
        primaryKey: false,
        required: true,
        references: "account.id",
      },
      {
        name: "name",
        dataType: "text",
        primaryKey: false,
        required: true,
        references: null,
      },
    ],
  },
  {
    name: "channel",
    rows: 214,
    columns: [
      {
        name: "id",
        dataType: "uuid",
        primaryKey: true,
        required: true,
        references: null,
      },
      {
        name: "space_id",
        dataType: "uuid",
        primaryKey: false,
        required: true,
        references: "space.id",
      },
      {
        name: "topic",
        dataType: "text",
        primaryKey: false,
        required: false,
        references: null,
      },
    ],
    note: "one row per room inside a space",
  },
  {
    name: "message",
    rows: 918432,
    columns: [
      {
        name: "id",
        dataType: "bigint",
        primaryKey: true,
        required: true,
        references: null,
      },
      {
        name: "channel_id",
        dataType: "uuid",
        primaryKey: false,
        required: true,
        references: "channel.id",
      },
      {
        name: "author_id",
        dataType: "uuid",
        primaryKey: false,
        required: true,
        references: "account.id",
      },
      {
        name: "sent_at",
        dataType: "timestamptz",
        primaryKey: false,
        required: true,
        references: null,
      },
    ],
  },
]

export const engines = [
  { name: "PostgreSQL", icon: "simple-icons:postgresql", note: "rustls" },
  { name: "MySQL", icon: "simple-icons:mysql", note: "native wire" },
  { name: "SQLite", icon: "simple-icons:sqlite", note: "file" },
  { name: "DuckDB", icon: "simple-icons:duckdb", note: "bundled" },
  { name: "Supabase", icon: "simple-icons:supabase", note: "postgres wire" },
  { name: "GreptimeDB", icon: "simple-icons:greptimedb", note: "port 4003" },
  {
    name: "ClickHouse",
    icon: "simple-icons:clickhouse",
    note: "native client",
  },
  { name: "Turso", icon: "simple-icons:turso", note: "libsql" },
  { name: "InfluxDB 3", icon: "simple-icons:influxdb", note: "arrow flight" },
  { name: "InfluxDB 2", icon: "simple-icons:influxdb", note: "flux" },
  { name: "Snowflake", icon: "simple-icons:snowflake", note: "account auth" },
  { name: "Neo4j", icon: "simple-icons:neo4j", note: "bolt" },
  { name: "FalkorDB", icon: "simple-icons:redis", note: "cypher" },
  { name: "Cloudflare D1", icon: "simple-icons:cloudflare", note: "http api" },
]

export const ask = [
  { role: "you", text: "busiest channels this week" },
  {
    role: "sql",
    text: `select channel.topic, count(*) as messages
from message
join channel on channel.id = message.channel_id
where message.sent_at > now() - interval '7 days'
group by channel.topic
order by messages desc`,
  },
]
