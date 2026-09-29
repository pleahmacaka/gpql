import { board, rem, splitReference } from "@gpql/ui"
import { listen } from "@tauri-apps/api/event"
import { asc, desc, eq, like } from "drizzle-orm"

import type { Side } from "$lib/ai/chat.svelte"
import { local } from "$lib/db/client"
import { migrate } from "$lib/db/migrate"
import {
  chatLog,
  favorite,
  preference,
  queryRun,
  recent,
  savedQuery,
  syncTombstone,
} from "$lib/db/schema"
import { ErdDocument, type ErdHost } from "$lib/erd/document.svelte"
import * as m from "$lib/paraglide/messages"
import { getLocale, type Locale, setLocale } from "$lib/paraglide/runtime"
import { bury, site, sync, unbury } from "$lib/sync/client"
import type {
  BackendInfo,
  Credential,
  Discovery,
  ErdRoom,
  LoginDetails,
  Mode,
  ObjectKind,
  Provider,
  SessionConfig,
  Tab,
} from "$lib/types"
import * as api from "./commands"
import { blankConfig } from "./commands"
import {
  BLANK,
  Connection,
  type ConnectionHost,
  roomOf,
} from "./connection.svelte"
import { foldersOf } from "./connections"
import { diffSchemas } from "./diff"
import { friendly, hint } from "./errors"
import { nounsFor } from "./nouns"

const PAGE = 1000
const ERD_DRAFT = "erd:draft"
const LIMITS = [200, 500, 1000, 5000, 20000]
const WINDOW = 5
const WINDOWS = [0, 1, 5, 15, 30]
const SIDES: Side[] = ["left", "center", "right"]

export const schemes = ["system", "light", "dark"] as const

export type Scheme = (typeof schemes)[number]

function readScheme(value: string | undefined): Scheme {
  return schemes.includes(value as Scheme) ? (value as Scheme) : "system"
}

function readSide(value: string | undefined): Side {
  return SIDES.find(side => side === value) ?? "right"
}

function why(failure: string) {
  return /auth|password|credential|401|403|unauthor|permission|refused the/i.test(
    failure,
  )
    ? "refused"
    : "down"
}

function hopping(config: SessionConfig) {
  return (config.tunnel?.host ?? "").trim() !== ""
}

const now = () => Math.floor(Date.now() / 1000)

// a command line keeps a quoted path such as "C:\Program Files\..." whole
function words(line: string) {
  const out: string[] = []
  let word = ""
  let quoted = false
  let started = false

  for (const char of line.trim()) {
    if (char === '"') {
      quoted = !quoted
      started = true

      continue
    }

    if (char.trim() === "" && !quoted) {
      if (started) {
        out.push(word)
      }

      word = ""
      started = false

      continue
    }

    word += char
    started = true
  }

  if (started) {
    out.push(word)
  }

  return out
}

function configFrom(login: LoginDetails, readOnly: boolean): SessionConfig {
  return {
    ...blankConfig(login.kind),
    kind: login.kind,
    host: login.host,
    port: login.port,
    user: login.user,
    password: login.password,
    database: login.database,
    path: login.path,
    url: login.endpoint,
    token: login.token,
    tls: login.tls,
    warehouse: login.warehouse,
    schema: login.schema,
    tunnel: login.tunnel ?? blankConfig(login.kind).tunnel,
    readOnly,
  }
}

export class Workspace {
  erd = $state<ErdDocument | null>(null)
  erdDraft = $state(false)
  tab = $state<Tab>("data")

  scheme = $state<Scheme>("system")
  systemDark = $state(false)
  compact = $state(false)
  readOnly = $state(true)
  writeWindow = $state(WINDOW)
  picked = $state("")
  locale = $state(getLocale())
  fuse: number | null = null
  acrylic = $state(false)
  autoscan = $state(true)
  motion = $state(true)
  ai = $state(true)
  aiGroups = $state(false)
  minimap = $state(true)
  rowLimit = $state(PAGE)
  settled = $state(true)
  texture = $state(35)
  previewWrites = $state(true)
  orbSide = $state<Side>("right")
  pretty = $state(false)

  connections = $state<Connection[]>([])
  activeId = $state<string | null>(null)
  asideWidth = $state(256)
  startup = $state<"last" | "recent">("last")

  recents = $state<(typeof recent.$inferSelect)[]>([])
  connectionView = $state<"list" | "grid">("list")
  finding = $state(false)
  notice = $state("")
  error = $state("")
  ddl = $state<{ name: string; kind?: ObjectKind; text: string } | null>(null)
  closing = $state<{
    kind: "session" | "erd"
    id: string
    label: string
  } | null>(null)

  found = $state<Discovery[]>([])
  scanning = $state(false)
  tailnet = $state<Discovery[]>([])
  tailnetError = $state("")
  scanningTailnet = $state(false)
  presets = $state<Credential[]>([])
  catalog = $state<BackendInfo[]>([])
  rooms = $state<ErdRoom[]>([])
  adding = $state(false)
  unreachable = $state<Record<string, string>>({})
  dialing = $state<string | null>(null)
  editing = $state<string | null>(null)
  providers = $state<Provider[]>([])
  signedIn = $state(false)

  languageServers = $state<Record<string, string>>({})
  servers = $state<string[]>([])
  lspError = $state("")

  busy = $state(false)

  private screen = $state<Mode>("recent")
  private dialingIn = $state(false)
  private lspTried = new Set<string>()
  private ddlFor = $state<string | null>(null)
  private closed: ((done: boolean) => void) | null = null

  private erdHost: ErdHost = {
    keepDraft: async text => {
      await this.remember(ERD_DRAFT, text)
      this.erdDraft = true
    },
    dropDraft: async () => {
      await local.delete(preference).where(eq(preference.key, ERD_DRAFT))
      this.erdDraft = false
    },
    saved: async doc => {
      await this.noteRecent({
        url: doc.path,
        kind: "erd",
        label: doc.name,
        detail: doc.path,
        openedAt: now(),
      })
      await this.reloadRecents()
    },
  }

  private wiring: ConnectionHost = {
    pageSize: () => this.rowLimit,
    backend: kind => this.catalog.find(entry => entry.id === kind),
    provider: () => this.model,
    remember: (key, value) => this.remember(key, value),
    steer: async (connection, move) => {
      switch (move.go) {
        case "query":
          this.tab = "query"
          await connection.query.replace(move.sql)
          connection.query.spot = true
          break
        case "data":
          this.tab = "data"
          await connection.select(move.table)
          break
        case "schema":
          this.tab = "schema"
          board.focus(move.table)
          break
        case "chat":
          break
      }
    },
    preview: () => this.previewWrites,
    setPreview: async on => {
      this.previewWrites = on
      await this.remember("previewWrites", on ? "on" : "off")
    },
    side: () => this.orbSide,
    activity: () => this.countDown(),
    report: failure => {
      this.error = friendly(failure)
    },
  }

  private idle = new Connection(BLANK, this.wiring)

  // leaving the connection form ends an edit, so a stale one never deletes it
  get mode() {
    return this.screen
  }

  set mode(next: Mode) {
    if (next !== "new") {
      this.editing = null
    }

    this.screen = next
  }

  get connecting() {
    return this.dialingIn
  }

  get errorHint() {
    return hint(this.error)
  }

  get ddlLoading() {
    return this.ddl !== null && this.ddlFor === this.ddl.name
  }

  set connecting(on: boolean) {
    this.editing = null
    this.dialingIn = on
  }

  get active(): Connection | null {
    return (
      this.connections.find(entry => entry.id === this.activeId) ??
      this.connections[0] ??
      null
    )
  }

  // the app is written against "the connection you are looking at"; the tab
  // strip is the only thing that needs to know there are several
  get session() {
    return this.active?.handle ?? null
  }

  get nouns() {
    return nounsFor(this.session?.kind ?? "")
  }

  get tables() {
    return this.active?.tables ?? []
  }

  get objects() {
    return this.active?.objects ?? []
  }

  get schema() {
    return this.active?.schema ?? []
  }

  get schemaNames() {
    return this.active?.schemaNames ?? []
  }

  get schemaPicked() {
    return this.active?.schemaPicked ?? ""
  }

  get favorites() {
    return this.active?.favorites ?? []
  }

  get shared() {
    return this.active?.shared ?? null
  }

  get browse() {
    return (this.active ?? this.idle).browse
  }

  get query() {
    return (this.active ?? this.idle).query
  }

  get writes() {
    return (this.active ?? this.idle).writes
  }

  get chat() {
    return (this.active ?? this.idle).chat
  }

  get keyColumns() {
    return this.active?.keyColumns ?? []
  }

  get columnTypes() {
    return this.active?.columnTypes ?? {}
  }

  get references() {
    return this.active?.references ?? {}
  }

  get writable() {
    return this.active?.writable ?? false
  }

  get dialect() {
    const backend = this.catalog.find(entry => entry.id === this.session?.kind)

    return backend?.dialect ?? "sql"
  }

  suggest(prefix: string) {
    const needle = prefix.toLowerCase()
    const words = new Set<string>()

    for (const table of this.tables) {
      words.add(table.name)
    }

    for (const table of this.schema) {
      for (const column of table.columns) {
        words.add(column.name)
      }
    }

    for (const column of this.browse.result?.columns ?? []) {
      words.add(column)
    }

    return [...words]
      .filter(word => word.toLowerCase().startsWith(needle) && word !== prefix)
      .sort()
      .slice(0, 30)
      .map(label => ({ label, detail: "schema", kind: 5 }))
  }

  get dark() {
    return this.scheme === "system" ? this.systemDark : this.scheme === "dark"
  }

  get theme() {
    return this.dark ? "gpql-dark" : "gpql"
  }

  get density() {
    return this.compact ? "py-1" : "py-2"
  }

  get rowHeight() {
    return rem(this.compact ? 1.5 : 2)
  }

  async boot() {
    listen<{ session: string; topic: string }>("catalog-changed", event => {
      this.connections
        .find(entry => entry.id === event.payload.session)
        ?.refreshSoon(event.payload.topic)
    })

    await migrate()
    await this.scrubRecents()

    const settings = await this.settingsMap()

    this.watchSystemScheme()
    this.applySettings(settings)
    this.readOnly = true
    this.settled = settings.get("settled") === "yes"
    this.asideWidth = Number(settings.get("asideWidth")) || this.asideWidth
    this.pretty = settings.get("pretty") === "on"
    this.erdDraft = settings.has(ERD_DRAFT)

    for (const [key, value] of settings) {
      if (key.startsWith("lsp:")) {
        this.languageServers[key.slice(4)] = value
      }
    }

    const loads = await Promise.allSettled([
      api.run(api.resetSessions()),
      this.paint(),
      this.reloadRecents(),
      this.idle.chat.reload(),
      this.idle.query.reload(),
      this.reloadCatalog(),
      this.reloadPresets(),
      this.reloadProviders(),
      this.refreshAccount(),
    ])

    this.error = [
      this.error,
      ...loads.flatMap(load =>
        load.status === "rejected" ? [friendly(String(load.reason))] : [],
      ),
    ]
      .filter(line => line !== "")
      .join("\n")

    const moved = await api.run(api.savedLoginsMoved()).catch(() => null)

    if (moved) {
      this.notice = m.logins_moved({ path: moved })
    }

    const [last] = [...this.recents].sort((a, b) => b.openedAt - a.openedAt)

    if (this.settled && this.startup === "last" && last) {
      void this.resume(last.url, last.kind)
    }
  }

  private async settingsMap() {
    const stored = await local.select().from(preference)

    return new Map(stored.map(row => [row.key, row.value]))
  }

  private applySettings(settings: Map<string, string>) {
    this.scheme = readScheme(settings.get("scheme"))
    this.compact = settings.get("compact") === "on"
    this.writeWindow = Number(settings.get("writeWindow") ?? WINDOW) || 0
    this.picked = settings.get("model") ?? ""
    this.acrylic = settings.get("acrylic") === "on"
    this.autoscan = settings.get("autoscan") !== "off"
    this.motion = settings.get("motion") !== "off"
    this.ai = settings.get("ai") !== "off"
    this.aiGroups = settings.get("aiGroups") === "on"
    this.minimap = settings.get("minimap") !== "off"
    this.rowLimit = Number(settings.get("rowLimit") ?? PAGE) || PAGE
    this.startup = settings.get("startup") === "recent" ? "recent" : "last"
    this.connectionView =
      settings.get("connectionView") === "grid" ? "grid" : "list"
    this.previewWrites = settings.get("previewWrites") !== "off"
    this.orbSide = readSide(settings.get("orbSide"))
    this.texture = Number(settings.get("texture") ?? 35)
  }

  // a password typed into a url used to land in the connection's address
  private async scrubRecents() {
    const rows = await local.select().from(recent)
    const taken = new Set(rows.map(row => row.url))

    for (const row of rows) {
      const url = api.stripSecrets(row.url)
      const detail = api.stripSecrets(row.detail)

      if (url === row.url && detail === row.detail) {
        continue
      }

      if (url === row.url) {
        await local.update(recent).set({ detail }).where(eq(recent.url, url))

        continue
      }

      if (taken.has(url)) {
        await local.delete(recent).where(eq(recent.url, row.url))
      } else {
        await local
          .update(recent)
          .set({ url, detail })
          .where(eq(recent.url, row.url))
      }

      taken.add(url)
    }
  }

  async setStartup(mode: "last" | "recent") {
    this.startup = mode
    await this.remember("startup", mode)
  }

  async setOrbSide(side: Side) {
    this.orbSide = side
    await this.remember("orbSide", side)
  }

  async setAsideWidth(width: number) {
    this.asideWidth = width
    await this.remember("asideWidth", String(width))
  }

  async setPretty(on: boolean) {
    this.pretty = on
    await this.remember("pretty", on ? "on" : "off")
  }

  async remember(key: string, value: string) {
    const updatedAt = now()

    await local
      .insert(preference)
      .values({ key, value, updatedAt })
      .onConflictDoUpdate({ target: preference.key, set: { value, updatedAt } })
  }

  async setScheme(scheme: Scheme) {
    this.scheme = scheme
    await this.remember("scheme", scheme)
    await this.paint()
  }

  async toggle(
    key:
      | "dark"
      | "compact"
      | "readOnly"
      | "acrylic"
      | "autoscan"
      | "motion"
      | "ai"
      | "aiGroups"
      | "minimap",
  ): Promise<void> {
    if (key === "dark") {
      return this.setScheme(this.dark ? "light" : "dark")
    }

    if (key === "readOnly") {
      return this.setReadOnly(!this.readOnly)
    }

    this[key] = !this[key]
    await this.remember(key, this[key] ? "on" : "off")

    if (key === "acrylic") {
      await this.paint()
    }
  }

  async setReadOnly(on: boolean) {
    const failures: string[] = []

    this.readOnly = on
    await this.remember("readOnly", on ? "on" : "off")

    for (const entry of this.connections) {
      try {
        await api.run(api.setReadOnly(entry.id, on))
        entry.handle = { ...entry.handle, readOnly: on }
      } catch (failure) {
        entry.handle = { ...entry.handle, readOnly: true }
        failures.push(`${entry.label}: ${failure}`)
      }
    }

    if (failures.length > 0) {
      this.error = failures.join("\n")
    }

    this.countDown()
  }

  // writes lock again only after a whole window passes with nothing written
  countDown() {
    if (this.fuse !== null) {
      clearTimeout(this.fuse)
      this.fuse = null
    }

    if (this.readOnly || this.writeWindow === 0) {
      return
    }

    this.fuse = window.setTimeout(
      () => {
        if (!this.readOnly) {
          void this.setReadOnly(true)
        }
      },
      this.writeWindow * 60 * 1000,
    )
  }

  async setWriteWindow(minutes: number) {
    this.writeWindow = minutes
    await this.remember("writeWindow", String(minutes))
    this.countDown()
  }

  async settle() {
    this.settled = true
    await this.remember("settled", "yes")
  }

  watchSystemScheme() {
    if (typeof window === "undefined" || !window.matchMedia) {
      return
    }

    const query = window.matchMedia("(prefers-color-scheme: dark)")

    this.systemDark = query.matches
    query.addEventListener("change", event => {
      this.systemDark = event.matches

      if (this.scheme === "system") {
        void this.paint()
      }
    })
  }

  get windows() {
    return WINDOWS
  }

  get limits() {
    return LIMITS
  }

  async setRowLimit(rows: number) {
    this.rowLimit = rows
    await this.remember("rowLimit", String(rows))

    if (this.browse.table) {
      await this.select(this.browse.table)
    }
  }

  async saveLayout(layout: {
    spots: Record<string, { x: number; y: number }>
    groups: { id: string; name: string; tables: string[] }[]
  }) {
    if (!this.active) {
      return
    }

    await this.remember(`layout:${this.active.origin}`, JSON.stringify(layout))
  }

  async loadLayout(label: string) {
    const keys = [`layout:${this.active?.origin ?? ""}`, `layout:${label}`]
    const rows = await local
      .select()
      .from(preference)
      .where(like(preference.key, "layout:%"))
    const row = keys
      .map(key => rows.find(entry => entry.key === key))
      .find(entry => entry !== undefined)

    if (!row) {
      return { spots: {}, groups: [] }
    }

    try {
      return JSON.parse(row.value) as {
        spots: Record<string, { x: number; y: number }>
        groups: { id: string; name: string; tables: string[] }[]
      }
    } catch {
      return { spots: {}, groups: [] }
    }
  }

  async wipeLocal() {
    for (const table of [
      recent,
      savedQuery,
      preference,
      queryRun,
      chatLog,
      favorite,
      syncTombstone,
    ]) {
      await local.delete(table)
    }

    this.recents = []
    this.unreachable = {}

    for (const entry of [this.idle, ...this.connections]) {
      entry.query.saved = []
      entry.query.history = []
      entry.chat.saved = []
      entry.favorites = []
      entry.shared = null
    }

    return m.local_cleared()
  }

  async syncNow() {
    const note = await sync()

    this.applySettings(await this.settingsMap())
    await this.paint()
    await this.reloadRecents()

    for (const entry of [this.idle, ...this.connections]) {
      await entry.query.reload()
    }

    return note
  }

  async paint() {
    await api.run(api.setAcrylic(this.acrylic, this.dark))
  }

  async setTexture(amount: number) {
    this.texture = amount
    await this.remember("texture", String(amount))
  }

  async reloadRecents() {
    this.recents = await local
      .select()
      .from(recent)
      .orderBy(asc(recent.rank), desc(recent.openedAt))
      .limit(100)

    await this.sniff()
  }

  // dragging writes a rank for every row at once, so a later insert with the
  // default rank of zero still lands at the top where a new connection belongs
  async reorderRecents(urls: string[]) {
    this.recents = urls
      .map(url => this.recents.find(entry => entry.url === url))
      .filter(entry => entry !== undefined)

    for (const [at, entry] of this.recents.entries()) {
      await local
        .update(recent)
        .set({ rank: at + 1 })
        .where(eq(recent.url, entry.url))
    }
  }

  async groupRecent(url: string, folder: string) {
    const named = folder.trim()

    await local
      .update(recent)
      .set({ folder: named === "" ? null : named })
      .where(eq(recent.url, url))

    await this.reloadRecents()
  }

  get folders() {
    return foldersOf(this.recents)
  }

  async setConnectionView(view: "list" | "grid") {
    this.connectionView = view
    await this.remember("connectionView", view)
  }

  async sniff() {
    const items = this.recents.map(entry => ({
      url: entry.url,
      kind: entry.kind,
    }))

    if (items.length === 0) {
      return
    }

    const answers = await api.run(api.probeRecents(items)).catch(() => [])
    const found: Record<string, string> = {}

    items.forEach((item, index) => {
      const open = this.connections.some(entry => entry.origin === item.url)

      found[item.url] = open ? "" : (answers[index] ?? "")
    })

    this.unreachable = found
  }

  async setLanguageServer(dialect: string, line: string) {
    const trimmed = line.trim()
    const next = { ...this.languageServers }

    if (trimmed === "") {
      delete next[dialect]
      this.languageServers = next
      await local.delete(preference).where(eq(preference.key, `lsp:${dialect}`))
      await this.stopLanguageServer(dialect)

      return
    }

    this.languageServers = { ...next, [dialect]: trimmed }
    await this.remember(`lsp:${dialect}`, trimmed)
    await this.startLanguageServer(dialect)
  }

  async startLanguageServer(dialect: string) {
    const [program, ...args] = words(this.languageServers[dialect] ?? "")

    if (!program) {
      return
    }

    this.lspError = ""

    try {
      await api.run(api.lspStart(dialect, program, args))
    } catch (failure) {
      this.lspError = String(failure)
      this.error = this.lspError
    }

    await this.reloadServers()
  }

  async stopLanguageServer(dialect: string) {
    this.lspError = ""

    try {
      await api.run(api.lspStop(dialect))
    } catch (failure) {
      this.lspError = String(failure)
    }

    await this.reloadServers()
  }

  private async reloadServers() {
    try {
      this.servers = await api.run(api.lspRunning())
    } catch (failure) {
      this.lspError = String(failure)
    }
  }

  private autostart(dialect: string) {
    if (this.lspTried.has(dialect) || !this.languageServers[dialect]) {
      return
    }

    this.lspTried.add(dialect)
    void this.startLanguageServer(dialect)
  }

  private async attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (failure) {
      this.error = friendly(String(failure))
    }
  }

  async publish() {
    const active = this.active

    if (!active) {
      return
    }

    await this.attempt(async () => {
      const room = await api.run(
        api.publishSchema(
          site,
          active.label,
          active.id,
          active.shared?.id ?? null,
        ),
      )

      await active.keepShared(room)
    })
  }

  async setShareOpen(open: boolean) {
    const active = this.active
    const room = active?.shared

    if (!active || !room) {
      return
    }

    await this.attempt(async () => {
      const answer = await api.run(api.shareErd(site, room.id, open))

      await active.keepShared({ ...room, open: answer })
    })
  }

  async closeShare() {
    const room = this.active?.shared

    if (room) {
      await this.closeRoom(room.id)
    }
  }

  async loadRooms() {
    const base = site.replace(/\/+$/, "")

    await this.attempt(async () => {
      const rooms = await api.run(api.listErd(site))

      this.rooms = rooms.map(room => ({
        ...room,
        link: `${base}/erd/${room.id}`,
      }))
    })
  }

  async closeRoom(id: string) {
    await this.attempt(async () => {
      await api.run(api.closeErd(site, id))
      this.rooms = this.rooms.filter(room => room.id !== id)

      for (const entry of this.connections) {
        if (entry.shared?.id === id) {
          entry.shared = null
        }
      }

      const kept = await local
        .select()
        .from(preference)
        .where(like(preference.key, "erd:%"))

      for (const row of kept) {
        if (roomOf(row.value)?.id === id) {
          await local.delete(preference).where(eq(preference.key, row.key))
        }
      }
    })
  }

  async reloadCatalog() {
    this.catalog = await api.run(api.backends())
  }

  async reloadPresets() {
    await this.attempt(async () => {
      this.presets = await api.run(api.credentials())
    })
  }

  // the agent answers from the open database and steers the tabs, so outside a
  // live session there is nothing for it to read or move
  get agentReady() {
    return (
      this.settled &&
      !!this.session &&
      !this.connecting &&
      !this.adding &&
      !this.erd &&
      this.ai &&
      !!this.model
    )
  }

  get model() {
    return (
      this.providers.find(entry => entry.id === this.picked) ??
      this.providers[0] ??
      null
    )
  }

  speak(next: Locale) {
    setLocale(next, { reload: false })
    this.locale = next
  }

  async pick(id: string) {
    this.picked = id
    await this.remember("model", id)
  }

  async reloadProviders() {
    await this.attempt(async () => {
      this.providers = await api.run(api.providers())
    })
  }

  async ask(providerId: string, prompt: string) {
    const connection = this.active

    if (!connection || prompt.trim() === "") {
      return
    }

    await connection.query.compose(async () => {
      const provider = this.providers.find(entry => entry.id === providerId)

      if (!provider) {
        throw new Error("gpql.no_model")
      }

      const [{ writeSql }] = await Promise.all([
        import("$lib/ai/sql"),
        connection.loadSchema(),
      ])

      return writeSql(provider, prompt, connection.schema, connection.query.sql)
    })
  }

  async refreshAccount() {
    this.signedIn = (await api.run(api.accountToken())) !== null
  }

  async scan() {
    this.scanning = true

    try {
      this.found = await api.run(api.scanLocal())
    } catch (failure) {
      this.error = friendly(String(failure))
    } finally {
      this.scanning = false
    }
  }

  async scanTailnet() {
    this.scanningTailnet = true
    this.tailnetError = ""

    try {
      this.tailnet = await api.run(api.scanTailnet())
    } catch (failure) {
      this.tailnetError = String(failure)
    } finally {
      this.scanningTailnet = false
    }
  }

  // reading may fall back to any engine at the same address; forgetting may not
  private async loginFor(url: string, strict = false) {
    const logins = await api.run(api.savedLogins())
    const clean = api.stripSecrets(url)

    return (
      logins.find(login => login.url === url) ??
      logins.find(login => api.stripSecrets(login.url) === clean) ??
      (strict ? null : logins.find(login => api.sameTarget(login.url, url))) ??
      null
    )
  }

  private async dropRecent(url: string) {
    const login = await this.loginFor(url, true)

    await local.delete(recent).where(eq(recent.url, url))
    await bury("recent", url)

    if (login) {
      await api.run(api.forgetLogin(login.url))
    }
  }

  private async noteRecent(row: typeof recent.$inferInsert) {
    const { url, ...fields } = row

    await local
      .insert(recent)
      .values(row)
      .onConflictDoUpdate({ target: recent.url, set: fields })
    await unbury("recent", url)
  }

  async open(config: SessionConfig) {
    if (!(await this.leaveErd())) {
      return
    }

    this.busy = true
    this.error = ""

    try {
      const handle = await api.run(
        api.connect({ ...config, readOnly: this.readOnly }),
      )
      const url = api.describe(config)
      const detail = api.stripSecrets(handle.detail)
      const stale = this.editing

      await this.noteRecent({
        url,
        kind: handle.kind,
        label: handle.label,
        detail,
        tunnelled: hopping(config) ? 1 : 0,
        openedAt: now(),
      })

      if (stale && !api.sameTarget(stale, url)) {
        await this.dropRecent(stale)
      }

      const opened = new Connection({ ...handle, detail }, this.wiring, url)

      this.editing = null
      this.erd = null
      this.connections = [...this.connections, opened]
      this.activeId = opened.id
      this.adding = false
      this.connecting = false
      this.tab = "data"
      this.notice = ""
      this.ddl = null
      board.reset()

      await this.reloadRecents()
      await opened.adopt()
      void opened.chat.reload()
      this.autostart(opened.dialect)
      await opened.loadTables()
    } catch (failure) {
      this.error = friendly(String(failure))
      throw failure
    } finally {
      this.busy = false
    }
  }

  async resume(url: string, kind = "", force = false) {
    this.editing = null

    if (!force && this.unreachable[url]) {
      return
    }

    if (kind === "erd") {
      this.dialing = url

      try {
        await this.startErd(url, true)
        this.unreachable = { ...this.unreachable, [url]: "" }
      } catch {
        this.unreachable = { ...this.unreachable, [url]: "gone" }
      } finally {
        this.dialing = null
      }

      return
    }

    let details: LoginDetails | null

    try {
      const login = await this.loginFor(url)

      details = login && (await api.run(api.savedLogin(login.url)))
    } catch (failure) {
      this.error = friendly(String(failure))

      return
    }

    if (!details) {
      this.unreachable = { ...this.unreachable, [url]: "forgotten" }

      return
    }

    this.dialing = url

    try {
      await this.open(configFrom(details, this.readOnly))
      this.unreachable = { ...this.unreachable, [url]: "" }
    } catch (failure) {
      this.error = ""
      this.unreachable = { ...this.unreachable, [url]: why(String(failure)) }
    } finally {
      this.dialing = null
    }
  }

  async keepConnection(config: SessionConfig) {
    await api.run(api.saveConnection(config))

    const url = api.describe(config)
    const stale = this.editing
    const label = config.database || config.path || config.kind
    const detail =
      api.stripSecrets(config.url) || `${config.host}:${config.port}`

    if (stale && !api.sameTarget(stale, url)) {
      await this.dropRecent(stale)
    }

    await this.noteRecent({
      url,
      kind: config.kind,
      label,
      detail,
      tunnelled: hopping(config) ? 1 : 0,
      openedAt: now(),
    })

    this.editing = null
    await this.reloadRecents()

    return url
  }

  async settings(url: string) {
    try {
      const login = await this.loginFor(url)
      const details = login && (await api.run(api.savedLogin(login.url)))

      if (!details) {
        return null
      }

      this.editing = url

      return configFrom(details, this.readOnly)
    } catch (failure) {
      this.error = friendly(String(failure))

      return null
    }
  }

  async startErd(path: string | null, existing: boolean) {
    const active = this.active

    if (active && !(await this.requestClose(active.id))) {
      return
    }

    if (!(await this.leaveErd())) {
      return
    }

    if (path === null) {
      this.erd = ErdDocument.untitled(this.erdHost)
      this.connecting = false

      return
    }

    const doc = existing
      ? await ErdDocument.open(path, this.erdHost)
      : await ErdDocument.create(path, this.erdHost)

    this.erd = doc
    this.connecting = false
    await this.erdHost.saved(doc)
  }

  async restoreErdDraft() {
    const [draft] = await local
      .select()
      .from(preference)
      .where(eq(preference.key, ERD_DRAFT))

    if (!draft) {
      this.erdDraft = false

      return
    }

    const active = this.active

    if (active && !(await this.requestClose(active.id))) {
      return
    }

    if (!(await this.leaveErd())) {
      return
    }

    this.erd = ErdDocument.untitled(this.erdHost, draft.value)
    this.connecting = false
  }

  async discardErdDraft() {
    await this.erdHost.dropDraft()
  }

  async closeErd() {
    if (!(await this.leaveErd())) {
      return
    }

    this.erd = null
    this.mode = "recent"
    this.connecting = true
  }

  // an untitled diagram lives only in its recovery draft until it is saved
  private async leaveErd() {
    const doc = this.erd

    if (!doc?.untitled || !doc.dirty) {
      doc?.stopDrafting()

      return true
    }

    this.settleClose(false)
    this.closing = { kind: "erd", id: "", label: doc.name }

    return new Promise<boolean>(resolve => {
      this.closed = resolve
    })
  }

  async saveAndClose() {
    const doc = this.erd

    if (this.closing?.kind !== "erd" || !doc) {
      return
    }

    if (!(await doc.save())) {
      return
    }

    this.closing = null
    this.settleClose(true)
  }

  async requestClose(id: string) {
    const going = this.connections.find(entry => entry.id === id)

    if (!going) {
      return false
    }

    if (!going.openTransaction) {
      await this.close(id)

      return true
    }

    this.settleClose(false)
    this.closing = { kind: "session", id, label: going.label }

    return new Promise<boolean>(resolve => {
      this.closed = resolve
    })
  }

  async confirmClose() {
    const closing = this.closing

    if (!closing) {
      return
    }

    this.closing = null

    if (closing.kind === "erd") {
      this.erd?.stopDrafting()
      await this.erdHost.dropDraft()
      this.settleClose(true)

      return
    }

    await this.attempt(async () => {
      await api.run(api.endTransaction(closing.id, false))
    })
    await this.close(closing.id)
    this.settleClose(true)
  }

  cancelClose() {
    this.closing = null
    this.settleClose(false)
  }

  private settleClose(done: boolean) {
    this.closed?.(done)
    this.closed = null
  }

  async close(id = this.activeId) {
    const going = this.connections.find(entry => entry.id === id)

    if (!going) {
      return
    }

    await going.close()

    this.connections = this.connections.filter(entry => entry.id !== going.id)

    if (!this.connections.some(entry => entry.id === this.activeId)) {
      this.activeId = this.connections[0]?.id ?? null
      this.ddl = null
    }

    if (this.connections.length === 0) {
      this.readOnly = true
      this.countDown()
      this.finding = false
      this.mode = "recent"
      this.notice = ""
      this.ddl = null
      board.reset()
    }
  }

  show(id: string) {
    const target = this.connections.find(entry => entry.id === id)

    if (!target) {
      return
    }

    this.activeId = id
    this.adding = false
    this.editing = null
    this.tab = "data"
    this.ddl = null
    board.reset()
    void target.chat.reload()
  }

  diffAgainst(other: Connection) {
    const here = this.active

    if (here?.schemaState !== "ready" || other.schemaState !== "ready") {
      return null
    }

    const views = [...here.objects, ...other.objects]
      .filter(entry => entry.kind === "view")
      .map(entry => entry.name)

    return diffSchemas(other.schema, here.schema, {
      kind: other.handle.kind,
      views,
    })
  }

  // the draft changes the other database, so it opens in that one's editor
  async draftMigration(id: string, sql: string) {
    const target = this.connections.find(entry => entry.id === id)

    if (!target) {
      return
    }

    this.show(id)
    this.tab = "query"
    await target.query.replace(sql)
  }

  async useSchema(name: string) {
    await this.attempt(async () => {
      await this.active?.useSchema(name)
    })
  }

  async showDdl(name: string, kind?: ObjectKind, detail?: string) {
    const session = this.session

    if (!session) {
      return
    }

    this.tab = "data"
    this.ddl = { name, kind, text: "" }
    this.ddlFor = name

    try {
      const text = await api.run(api.objectDdl(session.id, name, kind, detail))

      if (this.ddl?.name === name) {
        this.ddl = { name, kind, text }
      }
    } catch (failure) {
      if (this.ddl?.name === name) {
        this.ddl = { name, kind, text: friendly(String(failure)) }
      }
    } finally {
      if (this.ddlFor === name) {
        this.ddlFor = null
      }
    }
  }

  resetCatalog() {
    this.notice = ""
    this.ddl = null
    this.active?.forgetCatalog()
    board.reset()
  }

  async loadTables() {
    await this.attempt(async () => {
      await this.active?.loadTables()
    })
  }

  async toggleFavorite(name: string) {
    await this.active?.toggleFavorite(name)
  }

  iconFor(kind: string) {
    const found = this.catalog.find(entry => entry.id === kind)

    return found?.icon ?? "lucide:database"
  }

  async jumpTo(column: string, value: string) {
    const target = this.references[column]

    if (!target) {
      return
    }

    const { table, column: key } = splitReference(target)

    if (!key) {
      return
    }

    this.tab = "data"
    this.ddl = null
    await this.browse.open(table, {
      [key]: { op: "eq", value, needsValue: true },
    })
  }

  async applyEdits(
    edits: {
      keys: Record<string, string | null>
      set: Record<string, string | null>
    }[],
  ) {
    this.busy = true
    this.error = ""

    try {
      return (await this.active?.applyEdits(edits)) ?? false
    } catch (failure) {
      this.error = friendly(String(failure))
      throw failure
    } finally {
      this.busy = false
    }
  }

  async select(table: string) {
    this.ddl = null
    await this.attempt(async () => {
      await this.active?.select(table)
    })
  }

  async loadSchema() {
    await this.active?.loadSchema()
  }

  async renameRecent(url: string, alias: string) {
    const name = alias.trim()

    await local
      .update(recent)
      .set({ alias: name === "" ? null : name })
      .where(eq(recent.url, url))

    await this.reloadRecents()
  }

  async forgetRecent(url: string) {
    await this.attempt(() => this.dropRecent(url))
    await this.reloadRecents()
  }
}

export const workspace = new Workspace()
