import type { SchemaColumn, SchemaTable } from "@gpql/ui"

import * as api from "$lib/session/commands"

export const ERD_EXTENSION = "gpqlerd"

export class ErdDocument {
  path = $state("")
  name = $state("")
  tables = $state<SchemaTable[]>([])
  selected = $state<string | null>(null)
  dirty = $state(false)
  failure = $state("")

  static async open(path: string) {
    const doc = new ErdDocument()

    doc.path = path
    doc.name = fileName(path)

    const text = await api.run(api.readDocument(path))
    const parsed = JSON.parse(text) as { tables?: SchemaTable[] }

    doc.tables = parsed.tables ?? []
    doc.selected = doc.tables[0]?.name ?? null

    return doc
  }

  static async create(path: string) {
    const doc = new ErdDocument()

    doc.path = path
    doc.name = fileName(path)
    await doc.save()

    return doc
  }

  async save() {
    try {
      await api.run(
        api.writeDocument(
          this.path,
          JSON.stringify({ tables: this.tables }, null, 2),
        ),
      )
      this.failure = ""
      this.dirty = false
    } catch (problem) {
      this.failure = String(problem)
    }
  }

  addTable() {
    const name = freeName(
      this.tables.map(table => table.name),
      "table",
    )

    this.tables = [...this.tables, { name, rows: 0, columns: [] }]
    this.selected = name
    this.touch()
  }

  duplicateTable(name: string) {
    const source = this.tables.find(table => table.name === name)

    if (!source) {
      return
    }

    const copy = {
      ...source,
      name: freeName(
        this.tables.map(table => table.name),
        name,
      ),
      columns: source.columns.map(column => ({ ...column })),
    }

    this.tables = [...this.tables, copy]
    this.selected = copy.name
    this.touch()
  }

  removeTable(name: string) {
    this.tables = this.tables
      .filter(table => table.name !== name)
      .map(table => ({
        ...table,
        columns: table.columns.map(column =>
          column.references?.split(".")[0] === name
            ? { ...column, references: null }
            : column,
        ),
      }))

    if (this.selected === name) {
      this.selected = this.tables[0]?.name ?? null
    }

    this.touch()
  }

  renameTable(from: string, to: string) {
    const clean = to.trim()

    if (clean === "" || this.tables.some(table => table.name === clean)) {
      return
    }

    this.tables = this.tables.map(table => ({
      ...table,
      name: table.name === from ? clean : table.name,
      columns: table.columns.map(column =>
        column.references?.split(".")[0] === from
          ? {
              ...column,
              references: `${clean}.${column.references.split(".")[1]}`,
            }
          : column,
      ),
    }))

    if (this.selected === from) {
      this.selected = clean
    }

    this.touch()
  }

  addColumn(table: string) {
    this.editTable(table, current => {
      const name = freeName(
        current.columns.map(column => column.name),
        "column",
      )

      return {
        ...current,
        columns: [
          ...current.columns,
          {
            name,
            dataType: "text",
            primaryKey: current.columns.length === 0,
            required: false,
            references: null,
          },
        ],
      }
    })
  }

  updateColumn(table: string, index: number, patch: Partial<SchemaColumn>) {
    this.editTable(table, current => ({
      ...current,
      columns: current.columns.map((column, spot) =>
        spot === index ? { ...column, ...patch } : column,
      ),
    }))
  }

  removeColumn(table: string, index: number) {
    this.editTable(table, current => ({
      ...current,
      columns: current.columns.filter((_, spot) => spot !== index),
    }))
  }

  private touch() {
    this.dirty = true
    void this.save()
  }

  private editTable(name: string, edit: (table: SchemaTable) => SchemaTable) {
    this.tables = this.tables.map(table =>
      table.name === name ? edit(table) : table,
    )

    this.touch()
  }
}

function fileName(path: string) {
  const leaf = path.split(/[\/]/).pop() ?? path

  return leaf.replace(new RegExp(`\.${ERD_EXTENSION}$`), "")
}

function freeName(taken: string[], stem: string) {
  let count = taken.length + 1

  while (taken.includes(`${stem}_${count}`)) {
    count += 1
  }

  return `${stem}_${count}`
}
