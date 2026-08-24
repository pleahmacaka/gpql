import type { SchemaTable, SessionDraft } from "@gpql/ui"

export const draft: SessionDraft = {
  kind: "postgres",
  host: "127.0.0.1",
  port: "5432",
  user: "postgres",
  password: "hunter2hunter2",
  database: "smartfarmics",
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
  "house",
  "measured_at",
  "leaf_count",
  "leaf_area_cm2",
  "note",
]

const houses = ["greenhouse 1", "greenhouse 2", "nursery"]

export const rows: (string | null)[][] = Array.from(
  { length: 240 },
  (_, index) => [
    String(index + 1),
    houses[index % houses.length],
    `2026-08-${String((index % 28) + 1).padStart(2, "0")}`,
    String(64 + ((index * 7) % 48)),
    (900 + ((index * 137) % 2400)).toFixed(1),
    index % 9 === 0 ? null : "steady",
  ],
)

export const tables: SchemaTable[] = [
  {
    name: "user",
    rows: 12,
    columns: [
      {
        name: "id",
        dataType: "text",
        primaryKey: true,
        required: true,
        references: null,
      },
      {
        name: "email",
        dataType: "text",
        primaryKey: false,
        required: true,
        references: null,
      },
    ],
  },
  {
    name: "farm",
    rows: 3,
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
        dataType: "text",
        primaryKey: false,
        required: true,
        references: "user.id",
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
    name: "house",
    rows: 7,
    columns: [
      {
        name: "id",
        dataType: "uuid",
        primaryKey: true,
        required: true,
        references: null,
      },
      {
        name: "farm_id",
        dataType: "uuid",
        primaryKey: false,
        required: true,
        references: "farm.id",
      },
    ],
    note: "one row per physical greenhouse",
  },
  {
    name: "leaf_measurement",
    rows: 2679,
    columns: [
      {
        name: "id",
        dataType: "integer",
        primaryKey: true,
        required: true,
        references: null,
      },
      {
        name: "house_id",
        dataType: "uuid",
        primaryKey: false,
        required: true,
        references: "house.id",
      },
      {
        name: "leaf_count",
        dataType: "integer",
        primaryKey: false,
        required: true,
        references: null,
      },
    ],
  },
]
