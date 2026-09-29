import { invoke } from "@tauri-apps/api/core"

import { applySchema } from "./client"

const files = import.meta.glob("../../../drizzle/*.sql", {
  eager: true,
  query: "?raw",
  import: "default",
}) as Record<string, string>

const LEDGER = "create table if not exists _migration (name text primary key)"

async function applied() {
  const rows = await invoke<string[][]>("local_query", {
    sql: "select name from _migration",
    params: [],
  })

  return new Set(rows.map(row => row[0]))
}

function atomic(name: string, statements: string) {
  return [
    "begin immediate;",
    `insert into _migration (name) values ('${name.replaceAll("'", "''")}');`,
    statements.replaceAll("--> statement-breakpoint", ""),
    "commit;",
  ].join("\n")
}

export async function migrate() {
  await applySchema(LEDGER)

  const done = await applied()

  for (const path of Object.keys(files).sort()) {
    const name = path.split("/").pop() ?? path

    if (done.has(name)) {
      continue
    }

    try {
      await applySchema(atomic(name, files[path]))
    } catch (failure) {
      // a failed batch leaves its transaction open on the shared connection
      await applySchema("rollback").catch(() => undefined)

      if (!(await applied()).has(name)) {
        throw failure
      }
    }
  }
}
