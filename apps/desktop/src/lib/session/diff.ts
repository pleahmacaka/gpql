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

function quote(name: string) {
  return `"${name.replaceAll('"', '""')}"`
}

function columnClause(column: ColumnInfo) {
  return `${quote(column.name)} ${column.dataType}${
    column.required ? " not null" : ""
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

// the sql is deliberately additive: new tables and columns are written out,
// while drops and retypes are left commented because they lose data
function statementsFor(entry: TableDiff, source: TableSchema[]) {
  const out: string[] = []

  if (entry.state === "added") {
    const table = source.find(item => item.name === entry.table)
    const columns = (table?.columns ?? []).map(columnClause)
    const keys = (table?.columns ?? [])
      .filter(column => column.primaryKey)
      .map(column => quote(column.name))

    if (keys.length > 0) {
      columns.push(`primary key (${keys.join(", ")})`)
    }

    out.push(
      `create table ${quote(entry.table)} (\n  ${columns.join(",\n  ")}\n);`,
    )

    return out
  }

  if (entry.state === "dropped") {
    out.push(`-- drop table ${quote(entry.table)}; -- removes every row`)

    return out
  }

  for (const column of entry.addedColumns) {
    out.push(
      `alter table ${quote(entry.table)} add column ${columnClause(column)};`,
    )
  }

  for (const column of entry.droppedColumns) {
    out.push(
      `-- alter table ${quote(entry.table)} drop column ${quote(column.name)}; -- loses data`,
    )
  }

  for (const change of entry.changedColumns) {
    out.push(
      `-- alter table ${quote(entry.table)} alter column ${quote(change.name)} -- ${change.was} -> ${change.now}`,
    )
  }

  return out
}

export function diffSchemas(
  left: TableSchema[],
  right: TableSchema[],
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

  const sql = tables.flatMap(entry => statementsFor(entry, right))

  return { tables, sql }
}
