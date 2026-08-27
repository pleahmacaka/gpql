import { expect, test } from "bun:test"
import {
  byLevel,
  columnOffset,
  relationCount,
} from "../../../packages/ui/src/schema/levels"
import type { SchemaTable as TableSchema } from "../../../packages/ui/src/types"

const column = (name: string, references: string | null = null) => ({
  name,
  dataType: "text",
  primaryKey: name === "id",
  required: true,
  references,
})

const tables: TableSchema[] = [
  { name: "user", rows: 0, columns: [column("id")] },
  {
    name: "account",
    rows: 0,
    columns: [column("id"), column("user_id", "user.id")],
  },
  {
    name: "session",
    rows: 0,
    columns: [column("id"), column("account_id", "account.id")],
  },
  {
    name: "loop",
    rows: 0,
    columns: [column("id"), column("parent_id", "loop.id")],
  },
]

test("tables sit one level past whatever they point at", () => {
  const columns = byLevel(tables)

  expect(columns.map(entry => entry.label)).toEqual([
    "referenced",
    "level 1",
    "level 2",
  ])
  expect(columns[0].tables.map(table => table.name)).toEqual(["user", "loop"])
  expect(columns[1].tables.map(table => table.name)).toEqual(["account"])
  expect(columns[2].tables.map(table => table.name)).toEqual(["session"])
})

test("every foreign key column counts as one relation", () => {
  expect(relationCount(tables)).toBe(3)
})

test("policy rows add height so cards cannot overlap", () => {
  const bare: TableSchema = { name: "users", rows: 0, columns: [column("id")] }
  const guarded: TableSchema = { ...bare, policies: ["policy a (ALL)"] }

  expect(columnOffset(guarded, 0)).toBeGreaterThan(columnOffset(bare, 0))
})

test("a multi-line note pushes columns further down than a single line", () => {
  const one: TableSchema = {
    name: "users",
    rows: 0,
    columns: [column("id")],
    note: "one",
  }
  const two: TableSchema = { ...one, note: ["one", "two"].join("\n") }

  expect(columnOffset(two, 0) - columnOffset(one, 0)).toBeGreaterThan(0)
})
