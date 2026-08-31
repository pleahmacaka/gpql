import { Database } from "bun:sqlite"
import { expect, test } from "bun:test"
import { readdirSync, readFileSync } from "node:fs"
import { join } from "node:path"

const folder = join(import.meta.dir, "..", "drizzle")

function fresh() {
  const db = new Database(":memory:")

  const names = readdirSync(folder)
    .filter((name: string) => name.endsWith(".sql"))
    .sort()

  for (const name of names) {
    const sql = readFileSync(join(folder, name), "utf8")

    for (const statement of sql.split("--> statement-breakpoint")) {
      if (statement.trim() !== "") {
        db.run(statement)
      }
    }
  }

  return db
}

function columns(db: Database, table: string) {
  return db
    .query(`pragma table_info(${table})`)
    .all()
    .map((row: unknown) => (row as { name: string }).name)
}

test("every migration applies in order onto an empty database", () => {
  const db = fresh()

  expect(columns(db, "recent").length).toBeGreaterThan(0)
  db.close()
})

test("a connection carries its group, its rank and its tunnel flag", () => {
  const db = fresh()
  const named = columns(db, "recent")

  expect(named).toContain("folder")
  expect(named).toContain("rank")
  expect(named).toContain("tunnelled")
  expect(named).toContain("alias")
  db.close()
})

test("a connection saved without them still reads back", () => {
  const db = fresh()

  db.run(
    "insert into recent (url, kind, label, detail, opened_at) values (?, ?, ?, ?, ?)",
    ["pg://a", "postgres", "app", "127.0.0.1:5432", 1],
  )

  const row = db.query("select * from recent").get() as {
    folder: string | null
    rank: number
    tunnelled: number
  }

  expect(row.folder).toBeNull()
  expect(row.rank).toBe(0)
  expect(row.tunnelled).toBe(0)
  db.close()
})

test("connections order by rank first and by last opened after", () => {
  const db = fresh()
  const rows: [string, number, number][] = [
    ["pg://old", 0, 10],
    ["pg://new", 0, 30],
    ["pg://ranked", 1, 20],
  ]

  for (const [url, rank, openedAt] of rows) {
    db.run(
      "insert into recent (url, kind, label, detail, rank, opened_at) values (?, 'postgres', ?, '', ?, ?)",
      [url, url, rank, openedAt],
    )
  }

  const order = db
    .query("select url from recent order by rank asc, opened_at desc")
    .all()
    .map((row: unknown) => (row as { url: string }).url)

  expect(order).toEqual(["pg://new", "pg://old", "pg://ranked"])
  db.close()
})
