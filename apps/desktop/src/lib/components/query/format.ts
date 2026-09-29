import type { SqlLanguage } from "sql-formatter"

const LANGUAGES: Record<string, SqlLanguage> = {
  postgres: "postgresql",
  supabase: "postgresql",
  supabase_api: "postgresql",
  greptimedb: "postgresql",
  mysql: "mysql",
  sqlite: "sqlite",
  turso: "sqlite",
  d1: "sqlite",
  duckdb: "duckdb",
  clickhouse: "clickhouse",
  snowflake: "snowflake",
}

let offered: (() => void) | null = null

export function formattable(dialect: string) {
  return dialect === "sql"
}

export async function tidy(sql: string, kind: string) {
  const { format } = await import("sql-formatter")

  return format(sql, { language: LANGUAGES[kind] ?? "sql" })
}

export function offerFormat(run: () => void) {
  offered = run

  return () => {
    if (offered === run) {
      offered = null
    }
  }
}

export function canFormat() {
  return offered !== null
}

export function formatNow() {
  offered?.()
}
