import { expect, test } from "bun:test"
import { byLevel, relationCount } from "../../../packages/ui/src/levels"
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
