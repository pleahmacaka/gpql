import type { ColumnInfo, TableSchema } from "$lib/types"

export type ColumnChange = {
  name: string
  was: string
  now: string
}

export type TableDiff = {
  table: string
  state: "added" | "dropped" | "changed"
  addedColumns: ColumnInfo[]
  droppedColumns: ColumnInfo[]
  changedColumns: ColumnChange[]
}

export type SchemaDiff = {
  tables: TableDiff[]
  sql: string[]
}

export type DraftTarget = {
  kind?: string
  views?: string[]
}

type Draft = {
  quote: (name: string) => string
  mysql: boolean
  views: Set<string>
}

function shape(column: ColumnInfo) {
  const parts = [column.dataType]

  if (column.required) {
    parts.push("not null")
  }

  if (column.primaryKey) {
    parts.push("primary key")
  }

  if (column.references) {
    parts.push(`-> ${column.references}`)
  }

  return parts.join(" ")
}

function draftFor(target: DraftTarget): Draft {
  const mysql = target.kind === "mysql"

  return {
    mysql,
    views: new Set(target.views ?? []),
    quote: mysql
      ? name => `\`${name.replaceAll("`", "``")}\``
      : name => `"${name.replaceAll('"', '""')}"`,
  }
}

function columnClause(draft: Draft, column: ColumnInfo, nullable = false) {
  const required = column.required && !nullable

  return `${draft.quote(column.name)} ${column.dataType}${
    required ? " not null" : ""
  }`
}

function compareTable(left: TableSchema, right: TableSchema): TableDiff | null {
  const before = new Map(left.columns.map(column => [column.name, column]))
  const after = new Map(right.columns.map(column => [column.name, column]))

  const addedColumns = right.columns.filter(column => !before.has(column.name))
  const droppedColumns = left.columns.filter(column => !after.has(column.name))

  const changedColumns: ColumnChange[] = []

  for (const [name, column] of after) {
    const older = before.get(name)

    if (older && shape(older) !== shape(column)) {
      changedColumns.push({ name, was: shape(older), now: shape(column) })
    }
  }

  if (
    addedColumns.length === 0 &&
    droppedColumns.length === 0 &&
    changedColumns.length === 0
  ) {
    return null
  }

  return {
    table: right.name,
    state: "changed",
    addedColumns,
    droppedColumns,
    changedColumns,
  }
}

function addColumn(draft: Draft, table: string, column: ColumnInfo) {
  const name = draft.quote(table)
  const out = [
    `alter table ${name} add column ${columnClause(draft, column, true)};`,
  ]

  if (column.required) {
    const tighten = draft.mysql
      ? `modify column ${columnClause(draft, column)}`
      : `alter column ${draft.quote(column.name)} set not null`

    out.push(
      `-- alter table ${name} ${tighten}; -- once existing rows have a value`,
    )
  }

  return out
}

function createTable(draft: Draft, table: string, source: TableSchema[]) {
  const columns = source.find(item => item.name === table)?.columns ?? []
  const clauses = columns.map(column => columnClause(draft, column))
  const keys = columns
    .filter(column => column.primaryKey)
    .map(column => draft.quote(column.name))

  if (keys.length > 0) {
    clauses.push(`primary key (${keys.join(", ")})`)
  }

  return [
    `create table ${draft.quote(table)} (\n  ${clauses.join(",\n  ")}\n);`,
  ]
}

function viewNote(draft: Draft, entry: TableDiff) {
  const name = draft.quote(entry.table)

  if (entry.state === "dropped") {
    return `-- drop view ${name};`
  }

  return `-- view ${name} differs; recreate it from its definition`
}

// the sql is deliberately additive: new tables and columns are written out,
// while drops and retypes are left commented because they lose data
function statementsFor(draft: Draft, entry: TableDiff, source: TableSchema[]) {
  const name = draft.quote(entry.table)

  if (draft.views.has(entry.table)) {
    return [viewNote(draft, entry)]
  }

  if (entry.state === "added") {
    return createTable(draft, entry.table, source)
  }

  if (entry.state === "dropped") {
    return [`-- drop table ${name}; -- removes every row`]
  }

  return [
    ...entry.addedColumns.flatMap(column =>
      addColumn(draft, entry.table, column),
    ),
    ...entry.droppedColumns.map(
      column =>
        `-- alter table ${name} drop column ${draft.quote(column.name)}; -- loses data`,
    ),
    ...entry.changedColumns.map(
      change =>
        `-- alter table ${name} alter column ${draft.quote(change.name)} -- ${change.was} -> ${change.now}`,
    ),
  ]
}

export function diffSchemas(
  left: TableSchema[],
  right: TableSchema[],
  target: DraftTarget = {},
): SchemaDiff {
  const before = new Map(left.map(table => [table.name, table]))
  const after = new Map(right.map(table => [table.name, table]))

  const tables: TableDiff[] = []

  for (const table of right) {
    const older = before.get(table.name)

    if (!older) {
      tables.push({
        table: table.name,
        state: "added",
        addedColumns: table.columns,
        droppedColumns: [],
        changedColumns: [],
      })

      continue
    }

    const changed = compareTable(older, table)

    if (changed) {
      tables.push(changed)
    }
  }

  for (const table of left) {
    if (!after.has(table.name)) {
      tables.push({
        table: table.name,
        state: "dropped",
        addedColumns: [],
        droppedColumns: table.columns,
        changedColumns: [],
      })
    }
  }

  tables.sort((a, b) => a.table.localeCompare(b.table))

  const draft = draftFor(target)
  const sql = tables.flatMap(entry => statementsFor(draft, entry, right))

  return { tables, sql }
}
