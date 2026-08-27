import { expect, test } from "bun:test"

import { diffSchemas } from "../src/lib/session/diff"
import type { ColumnInfo, TableSchema } from "../src/lib/types"

const column = (
  name: string,
  dataType = "text",
  extra: Partial<ColumnInfo> = {},
): ColumnInfo => ({
  name,
  dataType,
  primaryKey: false,
  required: false,
  references: null,
  ...extra,
})

const table = (name: string, columns: ColumnInfo[]): TableSchema => ({
  name,
  rows: 0,
  columns,
})

test("an unchanged schema produces nothing", () => {
  const shape = [table("book", [column("id", "int"), column("title")])]

  expect(diffSchemas(shape, shape).tables).toEqual([])
  expect(diffSchemas(shape, shape).sql).toEqual([])
})

test("a new table becomes a create statement with its key", () => {
  const before: TableSchema[] = []
  const after = [
    table("book", [
      column("id", "int", { primaryKey: true, required: true }),
      column("title", "text", { required: true }),
    ]),
  ]

  const { tables, sql } = diffSchemas(before, after)

  expect(tables[0].state).toBe("added")
  expect(sql[0]).toContain('create table "book"')
  expect(sql[0]).toContain('"id" int not null')
  expect(sql[0]).toContain('primary key ("id")')
})

test("a new column becomes an add column statement", () => {
  const before = [table("book", [column("id", "int")])]
  const after = [table("book", [column("id", "int"), column("isbn")])]

  const { tables, sql } = diffSchemas(before, after)

  expect(tables[0].state).toBe("changed")
  expect(tables[0].addedColumns.map(entry => entry.name)).toEqual(["isbn"])
  expect(sql).toEqual(['alter table "book" add column "isbn" text;'])
})

test("anything that would lose data is written commented out", () => {
  const before = [table("book", [column("id", "int"), column("junk")])]
  const after = [table("book", [column("id", "int")])]

  const { sql } = diffSchemas(before, after)

  expect(sql).toHaveLength(1)
  expect(sql[0].startsWith("--")).toBe(true)
  expect(sql[0]).toContain("loses data")
})

test("a dropped table is reported but never dropped for you", () => {
  const before = [table("gone", [column("id", "int")])]

  const { tables, sql } = diffSchemas(before, [])

  expect(tables[0].state).toBe("dropped")
  expect(sql[0].startsWith("-- drop table")).toBe(true)
})

test("a retyped column is spelled out both ways", () => {
  const before = [table("book", [column("price", "int")])]
  const after = [table("book", [column("price", "numeric")])]

  const { tables } = diffSchemas(before, after)

  expect(tables[0].changedColumns[0]).toMatchObject({
    name: "price",
    was: "int",
    now: "numeric",
  })
})

test("a column becoming required counts as a change", () => {
  const before = [table("book", [column("title")])]
  const after = [table("book", [column("title", "text", { required: true })])]

  expect(
    diffSchemas(before, after)["tables"][0].changedColumns[0].now,
  ).toContain("not null")
})

test("quotes in an identifier cannot break out of the statement", () => {
  const after = [table('od"d', [column("id", "int")])]

  expect(diffSchemas([], after).sql[0]).toContain('create table "od""d"')
})
