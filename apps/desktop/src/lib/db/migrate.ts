import { invoke } from "@tauri-apps/api/core"

import { applySchema } from "./client"

const files = import.meta.glob("../../../drizzle/*.sql", {
  eager: true,
  query: "?raw",
  import: "default",
}) as Record<string, string>

const LEDGER = "create table if not exists _migration (name text primary key)"

export async function migrate() {
  await applySchema(LEDGER)

  const applied = await invoke<string[][]>("local_query", {
    sql: "select name from _migration",
    params: [],
  })
  const done = new Set(applied.map(row => row[0]))

  for (const path of Object.keys(files).sort()) {
    const name = path.split("/").pop() ?? path

    if (done.has(name)) {
      continue
    }

    await applySchema(files[path].replaceAll("--> statement-breakpoint", ""))
    await invoke("local_query", {
      sql: "insert into _migration (name) values (?)",
      params: [name],
    })
  }
}
