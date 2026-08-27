import { and, eq } from "drizzle-orm"
import { Chat } from "$lib/ai/chat.svelte"
import type { PilotMove } from "$lib/ai/pilot"
import { local } from "$lib/db/client"
import { favorite } from "$lib/db/schema"
import type {
  DbObject,
  Provider,
  SessionHandle,
  TableInfo,
  TableSchema,
} from "$lib/types"

import { Browse } from "./browse.svelte"
import * as api from "./commands"
import { Query } from "./query.svelte"
import { Writes } from "./writes.svelte"

export type ConnectionHost = {
  pageSize: () => number
  provider: () => Provider | null
  remember: (key: string, value: string) => Promise<void>
  steer: (connection: Connection, move: PilotMove) => Promise<void>
}

// everything that belongs to one open database, so a second one can be opened
// beside it instead of replacing it
export class Connection {
  handle: SessionHandle
  origin = $state("")

  tables = $state<TableInfo[]>([])
  objects = $state<DbObject[]>([])
  schema = $state<TableSchema[]>([])
  schemaNames = $state<string[]>([])
  schemaPicked = $state("")
  favorites = $state<string[]>([])

  browse: Browse
  query: Query
  writes: Writes
  chat: Chat

  constructor(handle: SessionHandle, host: ConnectionHost) {
    this.handle = handle

    this.browse = new Browse({
      session: () => this.live,
      pageSize: host.pageSize,
      onCount: (table, rows) => this.noteCount(table, rows),
    })

    this.query = new Query({
      session: () => this.live,
      dialect: () => this.dialect,
      provider: host.provider,
      schema: async () => {
        await this.loadSchema()

        return this.schema
      },
    })

    this.writes = new Writes({
      session: () => this.live,
      remember: host.remember,
      refresh: async () => {
        if (this.browse.table) {
          await this.browse.reload()
        }
      },
    })

    this.chat = new Chat({
      provider: host.provider,
      context: async () => {
        await this.loadSchema()

        return { schema: this.schema, tables: this.tables }
      },
      steer: move => host.steer(this, move),
    })
  }

  get id() {
    return this.handle.id
  }

  // the placeholder connection stands in before anything is open, so it must
  // answer "not connected" rather than pretend
  private get live() {
    return this.handle.id === "" ? null : this.handle
  }

  get label() {
    return this.handle.label
  }

  get dialect() {
    return this.handle.kind === "neo4j" || this.handle.kind === "falkordb"
      ? "cypher"
      : "sql"
  }

  get writable() {
    return !this.handle.readOnly
  }

  get keyColumns() {
    const found = this.schema.find(entry => entry.name === this.browse.table)

    return (found?.columns ?? [])
      .filter(column => column.primaryKey)
      .map(column => column.name)
  }

  get columnTypes() {
    const found = this.schema.find(entry => entry.name === this.browse.table)

    return Object.fromEntries(
      (found?.columns ?? []).map(column => [column.name, column.dataType]),
    )
  }

  get references() {
    const found = this.schema.find(entry => entry.name === this.browse.table)

    return Object.fromEntries(
      (found?.columns ?? [])
        .filter(column => column.references)
        .map(column => [column.name, column.references as string]),
    )
  }

  private noteCount(table: string, rows: number) {
    const info = this.tables.find(entry => entry.name === table)
    const node = this.schema.find(entry => entry.name === table)

    if (info) {
      info.rows = rows
    }

    if (node) {
      node.rows = rows
    }
  }

  async loadTables() {
    this.tables = await api.run(api.tables(this.id))
    this.schemaNames = await api.run(api.schemas(this.id))
    this.objects = await api.run(api.objects(this.id))

    if (this.schemaPicked === "") {
      this.schemaPicked =
        this.schemaNames.find(name => name === "public") ??
        this.schemaNames[0] ??
        ""
    }

    await this.loadFavorites()

    const first = this.tables[0]

    if (first) {
      await this.select(first.name)
    }
  }

  described = $state(false)

  // the model only annotates what it is shown, and the notes stay in memory
  // rather than being written back to the database
  async describe(provider: Provider) {
    await this.loadSchema()

    const { describeTables } = await import("$lib/ai/advise")
    const notes = await describeTables(provider, this.schema)

    this.schema = this.schema.map(table =>
      notes[table.name] ? { ...table, note: notes[table.name] } : table,
    )
    this.described = true
  }

  async loadSchema() {
    if (this.schema.length > 0) {
      return
    }

    this.schema = await api.run(api.schema(this.id))
  }

  async select(table: string) {
    await this.browse.open(table)
    await this.loadSchema()
  }

  async useSchema(name: string) {
    if (name === this.schemaPicked) {
      return
    }

    await api.run(api.useSchema(this.id, name))
    this.schemaPicked = name
    this.forgetCatalog()
    await this.loadTables()
  }

  forgetCatalog() {
    this.described = false
    this.schema = []
    this.browse.reset()
    this.query.reset()
  }

  async loadFavorites() {
    const found = await local
      .select()
      .from(favorite)
      .where(eq(favorite.target, this.label))

    this.favorites = found.map(entry => entry.table)
  }

  async toggleFavorite(name: string) {
    if (this.favorites.includes(name)) {
      this.favorites = this.favorites.filter(entry => entry !== name)
      await local
        .delete(favorite)
        .where(and(eq(favorite.target, this.label), eq(favorite.table, name)))

      return
    }

    this.favorites = [...this.favorites, name]
    await local
      .insert(favorite)
      .values({ target: this.label, table: name })
      .onConflictDoNothing()
  }

  async applyEdits(
    edits: {
      keys: Record<string, string | null>
      set: Record<string, string | null>
    }[],
  ) {
    const table = this.browse.table

    if (!table || !(await this.writes.confirm(table, edits))) {
      return
    }

    await api.run(api.applyEdits(this.id, table, edits))
    this.writes.noteWrite()
    await this.select(table)
  }

  async close() {
    this.writes.reset()

    try {
      await api.run(api.disconnect(this.id))
    } catch {
      // the socket may already be gone; the handle is dropped either way
    }
  }
}

export const idle = new Connection(
  {
    id: "",
    label: "",
    detail: "",
    kind: "postgres",
    readOnly: true,
    sliceable: false,
    transactional: false,
  },
  {
    pageSize: () => 0,
    provider: () => null,
    remember: async () => {},
    steer: async () => {},
  },
)
