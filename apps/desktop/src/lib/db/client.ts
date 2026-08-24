import { invoke } from "@tauri-apps/api/core"
import { drizzle } from "drizzle-orm/sqlite-proxy"

import * as schema from "./schema"

export const local = drizzle(
  async (sql, params, method) => {
    const rows = await invoke<unknown[][]>("local_query", { sql, params })

    return { rows: method === "get" ? (rows[0] ?? []) : rows }
  },
  { schema },
)

export function applySchema(statements: string) {
  return invoke<void>("local_batch", { sql: statements })
}
