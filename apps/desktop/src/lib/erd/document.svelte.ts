import { type SchemaColumn, type SchemaTable, splitReference } from "@gpql/ui"
import { save as pickFile } from "@tauri-apps/plugin-dialog"

import * as m from "$lib/paraglide/messages"
import * as api from "$lib/session/commands"

export const ERD_EXTENSION = "gpqlerd"
export const ERD_FILTERS = [{ name: "GPQL ERD", extensions: [ERD_EXTENSION] }]

const DRAFT_PAUSE = 500

export type ErdHost = {
  keepDraft: (text: string) => Promise<void>
  dropDraft: () => Promise<void>
  saved: (doc: ErdDocument) => Promise<void>
}

export class ErdDocument {
  path = $state("")
  name = $state("")
  tables = $state<SchemaTable[]>([])
  selected = $state<string | null>(null)
  dirty = $state(false)
  failure = $state("")

  private host: ErdHost
  private drafting: number | null = null

  private constructor(host: ErdHost) {
    this.host = host
  }

  static untitled(host: ErdHost, draft = "") {
    const doc = new ErdDocument(host)

    doc.name = m.erd_untitled()
    doc.tables = draft === "" ? [] : tablesFrom(parsed(draft))
    doc.selected = doc.tables[0]?.name ?? null
    doc.dirty = doc.tables.length > 0

    return doc
  }

  static async open(path: string, host: ErdHost) {
    const doc = new ErdDocument(host)

    doc.path = path
    doc.name = fileName(path)

    const text = await api.run(api.readDocument(path))

    doc.tables = tablesFrom(JSON.parse(text))
    doc.selected = doc.tables[0]?.name ?? null

    return doc
  }

  static async create(path: string, host: ErdHost) {
    const doc = new ErdDocument(host)

    doc.path = path
    doc.name = fileName(path)
    await doc.write(path)

    return doc
  }

  get untitled() {
    return this.path === ""
  }

  async save() {
    return this.untitled ? this.saveAs() : this.write(this.path)
  }

  async saveAs() {
    const path = await pickFile({
      filters: ERD_FILTERS,
      defaultPath: `${this.name}.${ERD_EXTENSION}`,
    })

    if (!path || !(await this.write(path))) {
      return false
    }

    const drafted = this.untitled

    this.path = path
    this.name = fileName(path)
    this.stopDrafting()

    if (drafted) {
      await this.host.dropDraft()
    }

    await this.host.saved(this)

    return true
  }

  stopDrafting() {
    if (this.drafting !== null) {
      clearTimeout(this.drafting)
      this.drafting = null
    }
  }

  private async write(path: string) {
    try {
      await api.run(api.writeDocument(path, this.text()))
      this.failure = ""
      this.dirty = false

      return true
    } catch (problem) {
      this.failure = String(problem)

      return false
    }
  }

  private text() {
    return JSON.stringify({ tables: this.tables }, null, 2)
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
          column.references && splitReference(column.references).table === name
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
      columns: table.columns.map(column => {
        const target = column.references
          ? splitReference(column.references)
          : null

        return target?.table === from
          ? { ...column, references: `${clean}.${target.column}` }
          : column
      }),
    }))

    if (this.selected === from) {
      this.selected = clean
    }

    this.touch()
  }

  updateTable(name: string, patch: Partial<Pick<SchemaTable, "note">>) {
    this.editTable(name, current => ({ ...current, ...patch }))
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

    if (!this.untitled) {
      void this.write(this.path)

      return
    }

    this.stopDrafting()
    this.drafting = window.setTimeout(() => {
      this.drafting = null
      void this.host.keepDraft(this.text())
    }, DRAFT_PAUSE)
  }

  private editTable(name: string, edit: (table: SchemaTable) => SchemaTable) {
    this.tables = this.tables.map(table =>
      table.name === name ? edit(table) : table,
    )

    this.touch()
  }
}

function fileName(path: string) {
  const leaf = path.split(/[/\\]/).pop() ?? path

  return leaf.replace(new RegExp(`.${ERD_EXTENSION}$`), "")
}

function parsed(text: string): unknown {
  try {
    return JSON.parse(text)
  } catch {
    return null
  }
}

function freeName(taken: string[], stem: string) {
  let count = taken.length + 1

  while (taken.includes(`${stem}_${count}`)) {
    count += 1
  }

  return `${stem}_${count}`
}

function claim(taken: string[], name: string) {
  const free = taken.includes(name) ? freeName(taken, name) : name

  taken.push(free)

  return free
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null
}

function text(value: unknown) {
  return typeof value === "string" ? value : null
}

function list(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function columnsFrom(value: unknown): SchemaColumn[] {
  const taken: string[] = []

  return list(value).flatMap(column => {
    if (!isRecord(column) || typeof column.name !== "string") {
      return []
    }

    return [
      {
        name: claim(taken, column.name),
        dataType: text(column.dataType) ?? "text",
        primaryKey: column.primaryKey === true,
        required: column.required === true,
        references: text(column.references),
        note: text(column.note),
      },
    ]
  })
}

// the board keys its nodes by name, so a name repeated in a hand-edited file would crash it
function tablesFrom(value: unknown): SchemaTable[] {
  const taken: string[] = []

  return list(isRecord(value) ? value.tables : null).flatMap(table => {
    if (!isRecord(table) || typeof table.name !== "string") {
      return []
    }

    return [
      {
        name: claim(taken, table.name),
        rows: typeof table.rows === "number" ? table.rows : 0,
        columns: columnsFrom(table.columns),
        note: text(table.note),
        hints: list(table.hints).filter(hint => typeof hint === "string"),
        policies: list(table.policies).filter(line => typeof line === "string"),
      },
    ]
  })
}
