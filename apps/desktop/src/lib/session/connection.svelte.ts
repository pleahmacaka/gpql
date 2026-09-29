import { splitReference } from "@gpql/ui"
import { and, eq } from "drizzle-orm"
import { Chat, type Side } from "$lib/ai/chat.svelte"
import type { PilotMove } from "$lib/ai/pilot"
import { local } from "$lib/db/client"
import { favorite, preference, queryRun, savedQuery } from "$lib/db/schema"
import type {
  BackendInfo,
  DbObject,
  Provider,
  SchemaState,
  SessionHandle,
  SharedErd,
  Subscription,
  TableInfo,
  TableSchema,
} from "$lib/types"

import { Browse } from "./browse.svelte"
import * as api from "./commands"
import { Query } from "./query.svelte"
import { Writes } from "./writes.svelte"

export type ConnectionHost = {
  pageSize: () => number
  backend: (kind: string) => BackendInfo | undefined
  provider: () => Provider | null
  remember: (key: string, value: string) => Promise<void>
  steer: (connection: Connection, move: PilotMove) => Promise<void>
  preview: () => boolean
  setPreview: (on: boolean) => Promise<void>
  side: () => Side
  activity: () => void
  report: (failure: string) => void
}

export const BLANK: SessionHandle = {
  id: "",
  label: "",
  detail: "",
  kind: "postgres",
  readOnly: true,
  sliceable: false,
  transactional: false,
}

export function roomOf(value: string | undefined): SharedErd | null {
  if (!value) {
    return null
  }

  try {
    const parsed: unknown = JSON.parse(value)

    if (
      parsed &&
      typeof parsed === "object" &&
      "id" in parsed &&
      "link" in parsed &&
      "open" in parsed &&
      typeof parsed.id === "string" &&
      typeof parsed.link === "string" &&
      typeof parsed.open === "boolean"
    ) {
      return { id: parsed.id, link: parsed.link, open: parsed.open }
    }
  } catch {
    return null
  }

  return null
}

// everything that belongs to one open database, so a second one can be opened
// beside it instead of replacing it
export class Connection {
  handle = $state<SessionHandle>(BLANK)
  origin: string

  tables = $state<TableInfo[]>([])
  objects = $state<DbObject[]>([])
  schema = $state<TableSchema[]>([])
  schemaState = $state<SchemaState>("idle")
  schemaError = $state("")
  schemaNames = $state<string[]>([])
  schemaPicked = $state("")
  favorites = $state<string[]>([])
  shared = $state<SharedErd | null>(null)
  described = $state(false)

  subs = $state<Subscription[]>([])
  draft = $state({ topic: "", payload: "", qos: 1, retain: false })

  browse: Browse
  query: Query
  writes: Writes
  chat: Chat

  private host: ConnectionHost
  private schemaLoad: Promise<void> | null = null
  private catalogEra = 0
  private catalogTimer: ReturnType<typeof setTimeout> | undefined
  private catalogTopics = new Set<string>()
  private closed = false

  constructor(handle: SessionHandle, host: ConnectionHost, origin = "") {
    this.handle = handle
    this.host = host
    this.origin = origin

    this.browse = new Browse({
      session: () => this.live,
      pageSize: host.pageSize,
      onCount: (table, rows) => this.noteCount(table, rows),
    })

    this.query = new Query({
      session: () => this.live,
      target: () => this.origin,
      dialect: () => this.dialect,
      provider: host.provider,
      schema: async () => {
        await this.loadSchema()

        return this.schema
      },
      catalogChanged: () => this.refreshSoon(),
      wrote: () => {
        this.writes.noteWrite()
        host.activity()
      },
    })

    this.writes = new Writes({
      session: () => this.live,
      preview: host.preview,
      setPreview: host.setPreview,
      activity: host.activity,
      report: host.report,
      refresh: async () => {
        if (this.browse.table) {
          await this.browse.reload()
        }
      },
    })

    this.chat = new Chat({
      provider: host.provider,
      side: host.side,
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
    return this.handle.id === "" || this.closed ? null : this.handle
  }

  get label() {
    return this.handle.label
  }

  private get backend() {
    return this.host.backend(this.handle.kind)
  }

  get dialect() {
    return this.backend?.dialect ?? "sql"
  }

  get canExplain() {
    return this.live !== null && this.backend?.explain === true
  }

  get canAnalyze() {
    return this.canExplain && this.backend?.analyze === true
  }

  get writable() {
    return !this.handle.readOnly
  }

  get openTransaction() {
    return this.writes.open
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
    const listed = new Set(this.tables.map(entry => entry.name))
    const out: Record<string, string> = {}

    for (const column of found?.columns ?? []) {
      const target = column.references

      if (target && listed.has(splitReference(target).table)) {
        out[column.name] = target
      }
    }

    return out
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

  // mqtt topics can appear any time, so the backend signals a fresh one and
  // only the list is pulled again rather than the whole connect sequence
  refreshSoon(topic?: string) {
    if (this.closed) {
      return
    }

    if (topic) {
      this.catalogTopics.add(topic)
    }

    clearTimeout(this.catalogTimer)
    this.catalogTimer = setTimeout(() => {
      const arrived = this.catalogTopics

      this.catalogTopics = new Set()
      void this.refreshTables()

      if (this.browse.table && arrived.has(this.browse.table)) {
        void this.browse.reload()
      }
    }, 300)
  }

  // a load that started before the catalog moved must not land after it
  private moveCatalog() {
    this.schemaLoad = null

    return ++this.catalogEra
  }

  async refreshTables() {
    if (this.closed) {
      return
    }

    const era = this.moveCatalog()

    try {
      const [tables, objects] = await Promise.all([
        api.run(api.tables(this.id)),
        api.run(api.objects(this.id)),
      ])

      if (this.closed || era !== this.catalogEra) {
        return
      }

      this.tables = tables
      this.objects = objects
    } catch (failure) {
      if (era !== this.catalogEra) {
        return
      }

      this.host.report(String(failure))
    }

    const loaded =
      this.schemaState === "ready" || this.schemaState === "loading"

    this.schemaState = "idle"

    if (loaded) {
      await this.loadSchema()
    }
  }

  async loadTables() {
    this.tables = await api.run(api.tables(this.id))
    this.schemaNames = await api.run(api.schemas(this.id))
    this.objects = await api.run(api.objects(this.id))

    if (this.schemaNames.length > 0) {
      this.schemaPicked = await this.currentSchema()
    }

    await Promise.all([
      this.loadFavorites(),
      this.loadShared(),
      this.loadSubs(),
      this.query.reload(),
      this.query.reloadHistory(),
    ])

    const first = this.tables[0]

    if (first) {
      await this.select(first.name)
    }
  }

  private async currentSchema() {
    try {
      return await api.run(api.currentSchema(this.id))
    } catch {
      return this.schemaNames.includes("public") ? "public" : ""
    }
  }

  // the model only annotates what it is shown, and the notes stay in memory
  // rather than being written back to the database
  async describe(provider: Provider, abortSignal?: AbortSignal) {
    await this.loadSchema()

    const { describeTables } = await import("$lib/ai/advise")
    const notes = await describeTables(provider, this.schema, abortSignal)

    this.schema = this.schema.map(table =>
      notes[table.name] ? { ...table, note: notes[table.name] } : table,
    )
    this.described = true
  }

  loadSchema(force = false): Promise<void> {
    if (this.schemaLoad) {
      return this.schemaLoad
    }

    if (!force && this.schemaState !== "idle") {
      return Promise.resolve()
    }

    const load = this.fetchSchema(this.catalogEra).finally(() => {
      if (this.schemaLoad === load) {
        this.schemaLoad = null
      }
    })

    this.schemaLoad = load

    return load
  }

  private async fetchSchema(era: number) {
    this.schemaState = "loading"
    this.schemaError = ""

    try {
      const fresh = await api.run(api.schema(this.id))

      if (era !== this.catalogEra) {
        return
      }

      const notes = new Map(this.schema.map(table => [table.name, table.note]))

      this.schema = fresh.map(table =>
        notes.get(table.name)
          ? { ...table, note: notes.get(table.name) }
          : table,
      )
      this.schemaState = "ready"
    } catch (failure) {
      if (era !== this.catalogEra) {
        return
      }

      this.schemaError = String(failure)
      this.schemaState = "failed"
    }
  }

  async select(table: string) {
    await this.browse.open(table)
    await this.loadSchema()
  }

  async loadSubs() {
    if (this.handle.kind !== "mqtt") {
      return
    }

    this.subs = await api.run(api.mqttSubscriptions(this.id))
  }

  async mqttSubscribe(filter: string, qos: number) {
    await api.run(api.mqttSubscribe(this.id, filter, qos))
    await this.loadSubs()
  }

  async mqttUnsubscribe(filter: string) {
    await api.run(api.mqttUnsubscribe(this.id, filter))
    await this.loadSubs()
  }

  async mqttClear(topic: string) {
    await api.run(api.mqttClear(this.id, topic))
  }

  async mqttPublish(
    topic: string,
    payload: string,
    qos: number,
    retain: boolean,
  ) {
    await api.run(api.mqttPublish(this.id, topic, payload, qos, retain))
  }

  async s3Presign(bucket: string, key: string) {
    return api.run(api.s3Presign(this.id, bucket, key))
  }

  async s3Download(bucket: string, key: string, path: string) {
    return api.run(api.s3Download(this.id, bucket, key, path))
  }

  async s3Upload(bucket: string, key: string, path: string) {
    await api.run(api.s3Upload(this.id, bucket, key, path))
  }

  async s3Delete(bucket: string, key: string) {
    await api.run(api.s3Delete(this.id, bucket, key))
  }

  async s3Refresh(bucket: string) {
    await api.run(api.s3Refresh(this.id, bucket))
    await this.refreshTables()
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
    this.moveCatalog()
    this.described = false
    this.schema = []
    this.schemaState = "idle"
    this.schemaError = ""
    this.browse.reset()
    this.query.reset()
  }

  // rows saved before connections were keyed by address carry the database name
  async adopt() {
    if (this.origin === "" || this.label === "" || this.label === this.origin) {
      return
    }

    const legacy = await local
      .select()
      .from(favorite)
      .where(eq(favorite.target, this.label))

    for (const row of legacy) {
      await local
        .insert(favorite)
        .values({ target: this.origin, table: row.table })
        .onConflictDoNothing()
    }

    await local.delete(favorite).where(eq(favorite.target, this.label))
    await local
      .update(queryRun)
      .set({ target: this.origin })
      .where(eq(queryRun.target, this.label))
    await local
      .update(savedQuery)
      .set({ target: this.origin })
      .where(eq(savedQuery.target, this.label))

    const [layout] = await local
      .select()
      .from(preference)
      .where(eq(preference.key, `layout:${this.label}`))

    if (layout) {
      await local
        .insert(preference)
        .values({ key: `layout:${this.origin}`, value: layout.value })
        .onConflictDoNothing()
      await local
        .delete(preference)
        .where(eq(preference.key, `layout:${this.label}`))
    }
  }

  async loadFavorites() {
    const found = await local
      .select()
      .from(favorite)
      .where(eq(favorite.target, this.origin))

    this.favorites = found.map(entry => entry.table)
  }

  async toggleFavorite(name: string) {
    if (this.favorites.includes(name)) {
      this.favorites = this.favorites.filter(entry => entry !== name)
      await local
        .delete(favorite)
        .where(and(eq(favorite.target, this.origin), eq(favorite.table, name)))

      return
    }

    this.favorites = [...this.favorites, name]
    await local
      .insert(favorite)
      .values({ target: this.origin, table: name })
      .onConflictDoNothing()
  }

  async loadShared() {
    const [row] = await local
      .select()
      .from(preference)
      .where(eq(preference.key, `erd:${this.origin}`))

    this.shared = roomOf(row?.value)
  }

  async keepShared(room: SharedErd | null) {
    this.shared = room

    if (room) {
      await this.host.remember(`erd:${this.origin}`, JSON.stringify(room))

      return
    }

    await local
      .delete(preference)
      .where(eq(preference.key, `erd:${this.origin}`))
  }

  async applyEdits(
    edits: {
      keys: Record<string, string | null>
      set: Record<string, string | null>
    }[],
  ) {
    const table = this.browse.table

    if (!table || !(await this.writes.confirm(table, edits))) {
      return false
    }

    await api.run(api.applyEdits(this.id, table, edits))
    this.writes.noteWrite()
    this.host.activity()
    await this.select(table)

    return true
  }

  async close() {
    this.closed = true
    clearTimeout(this.catalogTimer)
    this.writes.reset()

    try {
      await api.run(api.disconnect(this.id))
    } catch {
      // the socket may already be gone; the handle is dropped either way
    }
  }
}
