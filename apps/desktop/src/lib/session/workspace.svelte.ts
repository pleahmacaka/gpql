import { desc, eq } from "drizzle-orm"

import { local } from "$lib/db/client"
import { migrate } from "$lib/db/migrate"
import { preference, recent, savedQuery } from "$lib/db/schema"
import { ErdDocument } from "$lib/erd/document.svelte"
import * as m from "$lib/paraglide/messages"
import { getLocale, setLocale } from "$lib/paraglide/runtime"
import type {
  BackendInfo,
  Credential,
  Discovery,
  Mode,
  Provider,
  QueryResult,
  SavedLogin,
  SessionConfig,
  SessionHandle,
  Tab,
  TableInfo,
  TableSchema,
} from "$lib/types"

import * as api from "./commands"
import { blankConfig } from "./commands"
import { friendly } from "./errors"

const PAGE = 1000
const LIMITS = [200, 500, 1000, 5000, 20000]
const WINDOW = 5
const WINDOWS = [0, 1, 5, 15, 30]

export type ChatTurn = { role: "you" | "agent"; text: string }

export const schemes = ["system", "light", "dark"] as const

export type Scheme = (typeof schemes)[number]

function readScheme(value: string | undefined): Scheme {
  return schemes.includes(value as Scheme) ? (value as Scheme) : "system"
}

function why(failure: string) {
  return /auth|password|credential|401|403|unauthor|permission|refused the/i.test(
    failure,
  )
    ? "refused"
    : "down"
}

function tail(url: string) {
  return url.slice(url.indexOf("://") + 3)
}

function configFrom(login: SavedLogin, readOnly: boolean): SessionConfig {
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
    readOnly,
  }
}

export class Workspace {
  session = $state<SessionHandle | null>(null)
  erd = $state<ErdDocument | null>(null)
  tab = $state<Tab>("data")
  mode = $state<Mode>("recent")

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

  tables = $state<TableInfo[]>([])
  selected = $state<string | null>(null)
  rows = $state<QueryResult | null>(null)

  schema = $state<TableSchema[]>([])

  sql = $state("")
  selection = $state({ start: 0, end: 0 })
  queryResult = $state<QueryResult | null>(null)
  queryError = $state<string | null>(null)
  queryRan = $state(false)

  recents = $state<(typeof recent.$inferSelect)[]>([])
  saved = $state<(typeof savedQuery.$inferSelect)[]>([])
  openQuery = $state<string | null>(null)
  autosaved = $state(false)
  finding = $state(false)

  found = $state<Discovery[]>([])
  scanning = $state(false)
  presets = $state<Credential[]>([])
  catalog = $state<BackendInfo[]>([])
  servers = $state<string[]>([])
  shared = $state("")
  connecting = $state(false)
  unreachable = $state<Record<string, string>>({})
  dialing = $state<string | null>(null)
  editing = $state<string | null>(null)
  chatOpen = $state(false)
  chatBusy = $state(false)
  chatError = $state("")
  chat = $state<ChatTurn[]>([])
  languageServers = $state<Record<string, string>>({})
  providers = $state<Provider[]>([])
  signedIn = $state(false)

  busy = $state(false)
  error = $state<string | null>(null)

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

    for (const column of this.rows?.columns ?? []) {
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
    return this.compact ? 26 : 34
  }

  async boot() {
    await migrate()

    const stored = await local.select().from(preference)
    const settings = new Map(stored.map(row => [row.key, row.value]))

    this.watchSystemScheme()
    this.scheme = readScheme(settings.get("scheme"))
    this.compact = settings.get("compact") === "on"
    this.readOnly = true
    this.writeWindow = Number(settings.get("writeWindow") ?? WINDOW) || 0
    this.picked = settings.get("model") ?? ""
    this.acrylic = settings.get("acrylic") === "on"
    this.autoscan = settings.get("autoscan") !== "off"
    this.motion = settings.get("motion") !== "off"
    this.ai = settings.get("ai") !== "off"
    this.aiGroups = settings.get("aiGroups") === "on"
    this.minimap = settings.get("minimap") !== "off"
    this.rowLimit = Number(settings.get("rowLimit") ?? PAGE) || PAGE
    this.settled = settings.get("settled") === "yes"

    for (const [key, value] of settings) {
      if (key.startsWith("lsp:")) {
        this.languageServers[key.slice(4)] = value
      }
    }
    this.texture = Number(settings.get("texture") ?? 35)

    await this.paint()

    await this.reloadRecents()
    await this.reloadSaved()
    await this.reloadCatalog()
    await this.reloadPresets()
    await this.reloadProviders()
    await this.refreshAccount()
  }

  async remember(key: string, value: string) {
    await local
      .insert(preference)
      .values({ key, value })
      .onConflictDoUpdate({ target: preference.key, set: { value } })
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

    this[key] = !this[key]
    await this.remember(key, this[key] ? "on" : "off")

    if (key === "readOnly") {
      if (this.session) {
        await api.run(api.setReadOnly(this.session.id, this.readOnly))
        this.session = { ...this.session, readOnly: this.readOnly }
      }

      this.countDown()
    }

    if (key === "acrylic") {
      await this.paint()
    }
  }

  countDown() {
    if (this.fuse !== null) {
      clearTimeout(this.fuse)
      this.fuse = null
    }

    if (this.readOnly || this.writeWindow === 0) {
      return
    }

    this.fuse = setTimeout(
      () => {
        if (!this.readOnly) {
          void this.toggle("readOnly")
        }
      },
      this.writeWindow * 60 * 1000,
    ) as unknown as number
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

    if (this.selected) {
      await this.select(this.selected)
    }
  }

  async saveLayout(layout: {
    spots: Record<string, { x: number; y: number }>
    groups: { id: string; name: string; tables: string[] }[]
  }) {
    if (!this.session) {
      return
    }

    await this.remember(`layout:${this.session.label}`, JSON.stringify(layout))
  }

  async loadLayout(label: string) {
    const [row] = await local
      .select()
      .from(preference)
      .where(eq(preference.key, `layout:${label}`))
      .limit(1)

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
    await local.delete(recent)
    await local.delete(savedQuery)
    await local.delete(preference)

    this.recents = []
    this.saved = []
    this.unreachable = {}

    return "local data cleared"
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
      .orderBy(desc(recent.openedAt))
      .limit(8)

    await this.sniff()
  }

  async sniff() {
    const items = this.recents.map(entry => ({
      url: entry.url,
      kind: entry.kind,
    }))

    if (items.length === 0) {
      return
    }

    const answers = await api.run(api.probeRecents(items))
    const found: Record<string, string> = {}

    items.forEach((item, index) => {
      found[item.url] = answers[index] ?? ""
    })

    this.unreachable = found
  }

  async reloadSaved() {
    this.saved = await local
      .select()
      .from(savedQuery)
      .orderBy(desc(savedQuery.savedAt))
  }

  async startLanguageServer(dialect: string, line: string) {
    const [program, ...args] = line.trim().split(/\s+/)

    if (!program) {
      await api.run(api.lspStop(dialect))
      this.servers = await api.run(api.lspRunning())

      return
    }

    await api.run(api.lspStart(dialect, program, args))
    this.servers = await api.run(api.lspRunning())
    await this.remember(`lsp:${dialect}`, line)
  }

  async publish(site: string) {
    if (!this.session) {
      return
    }

    this.shared = await api.run(
      api.publishSchema(site, this.session.label, this.session.id),
    )
  }

  async reloadCatalog() {
    this.catalog = await api.run(api.backends())
  }

  async reloadPresets() {
    this.presets = await api.run(api.credentials())
  }

  async say(prompt: string) {
    const text = prompt.trim()
    const provider = this.model

    if (text === "" || this.chatBusy) {
      return
    }

    if (!provider) {
      this.chatError = friendly("gpql.no_model")

      return
    }

    this.chat = [...this.chat, { role: "you", text }]
    this.chatBusy = true
    this.chatError = ""

    try {
      const { talk } = await import("$lib/ai/chat")
      const answer = await talk(provider, this.chat, this.schema)

      this.chat = [...this.chat, { role: "agent", text: answer }]
    } catch (failure) {
      this.chatError = friendly(String(failure))
    } finally {
      this.chatBusy = false
    }
  }

  get model() {
    return (
      this.providers.find(entry => entry.id === this.picked) ??
      this.providers[0] ??
      null
    )
  }

  speak(next: "en" | "ko") {
    setLocale(next, { reload: false })
    this.locale = next
  }

  async pick(id: string) {
    this.picked = id
    await this.remember("model", id)
  }

  async reloadProviders() {
    this.providers = await api.run(api.providers())
  }

  async ask(providerId: string, prompt: string) {
    if (!this.session || prompt.trim() === "") {
      return
    }

    this.busy = true
    this.queryError = null

    try {
      const provider = this.providers.find(entry => entry.id === providerId)

      if (!provider) {
        throw new Error("gpql.no_model")
      }

      const { writeSql } = await import("$lib/ai/sql")

      this.sql = await writeSql(provider, prompt, this.schema, this.sql)
      this.selection = { start: 0, end: 0 }
    } catch (failure) {
      this.queryError = friendly(String(failure))
    } finally {
      this.busy = false
    }
  }

  async refreshAccount() {
    this.signedIn = (await api.run(api.accountToken())) !== null
  }

  async scan() {
    this.scanning = true

    try {
      this.found = await api.run(api.scanLocal())
    } finally {
      this.scanning = false
    }
  }

  async open(config: SessionConfig) {
    this.busy = true
    this.error = null

    try {
      const handle = await api.run(
        api.connect({ ...config, readOnly: this.readOnly }),
      )
      const stamp = Math.floor(Date.now() / 1000)

      await local
        .insert(recent)
        .values({
          url: api.describe(config),
          kind: handle.kind,
          label: handle.label,
          detail: handle.detail,
          openedAt: stamp,
        })
        .onConflictDoUpdate({
          target: recent.url,
          set: { openedAt: stamp, label: handle.label, detail: handle.detail },
        })

      const stale = this.editing

      if (stale && stale !== api.describe(config)) {
        await local.delete(recent).where(eq(recent.url, stale))
        await api.run(api.forgetLogin(stale))
      }

      this.editing = null
      this.erd = null
      this.session = handle
      this.connecting = false
      this.tab = "data"
      await this.reloadRecents()
      await this.loadTables()
    } catch (failure) {
      this.error = friendly(String(failure))
      throw failure
    } finally {
      this.busy = false
    }
  }

  async resume(url: string, kind = "", force = false) {
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

    const logins = await api.run(api.savedLogins())
    const match =
      logins.find(login => login.url === url) ??
      logins.find(login => tail(login.url) === tail(url))

    if (!match) {
      this.unreachable = { ...this.unreachable, [url]: "forgotten" }

      return
    }

    this.dialing = url

    try {
      await this.open(configFrom(match, this.readOnly))
      this.unreachable = { ...this.unreachable, [url]: "" }
    } catch (failure) {
      this.error = null
      this.unreachable = { ...this.unreachable, [url]: why(String(failure)) }
    } finally {
      this.dialing = null
    }
  }

  async keepConnection(config: SessionConfig) {
    const url = await api.run(api.saveConnection(config))
    const stale = this.editing
    const stamp = Math.floor(Date.now() / 1000)
    const label = config.database || config.path || config.kind
    const detail = config.url || `${config.host}:${config.port}`

    if (stale && stale !== url) {
      await local.delete(recent).where(eq(recent.url, stale))
      await api.run(api.forgetLogin(stale))
    }

    await local
      .insert(recent)
      .values({ url, kind: config.kind, label, detail, openedAt: stamp })
      .onConflictDoUpdate({
        target: recent.url,
        set: { openedAt: stamp, label, detail },
      })

    this.editing = null
    await this.reloadRecents()

    return url
  }

  async settings(url: string) {
    const logins = await api.run(api.savedLogins())
    const match =
      logins.find(login => login.url === url) ??
      logins.find(login => tail(login.url) === tail(url))

    if (!match) {
      return null
    }

    this.editing = url

    return configFrom(match, this.readOnly)
  }

  async startErd(path: string, existing: boolean) {
    if (this.session) {
      await this.close()
    }

    this.connecting = false
    this.erd = existing
      ? await ErdDocument.open(path)
      : await ErdDocument.create(path)

    const stamp = Math.floor(Date.now() / 1000)

    await local
      .insert(recent)
      .values({
        url: path,
        kind: "erd",
        label: this.erd.name,
        detail: path,
        openedAt: stamp,
      })
      .onConflictDoUpdate({
        target: recent.url,
        set: { openedAt: stamp, label: this.erd.name, detail: path },
      })

    await this.reloadRecents()
  }

  closeErd() {
    this.erd = null
    this.mode = "recent"
    this.connecting = true
  }

  async close() {
    if (!this.session) {
      return
    }

    try {
      await api.run(api.disconnect(this.session.id))
    } catch {
      this.error = null
    }

    this.session = null
    this.readOnly = true
    this.countDown()
    this.tables = []
    this.schema = []
    this.rows = null
    this.selected = null
    this.queryResult = null
    this.queryRan = false
    this.mode = "recent"
  }

  async loadTables() {
    if (!this.session) {
      return
    }

    this.tables = await api.run(api.tables(this.session.id))
    const first = this.tables[0]

    if (first) {
      await this.select(first.name)
    }
  }

  iconFor(kind: string) {
    const found = this.catalog.find(entry => entry.id === kind)

    return found?.icon ?? "lucide:database"
  }

  get keyColumns() {
    const found = this.schema.find(entry => entry.name === this.selected)

    return (found?.columns ?? [])
      .filter(column => column.primaryKey)
      .map(column => column.name)
  }

  get writable() {
    return this.session !== null && !this.session.readOnly
  }

  async applyEdits(
    edits: {
      keys: Record<string, string | null>
      set: Record<string, string | null>
    }[],
  ) {
    if (!this.session || !this.selected) {
      return
    }

    this.busy = true
    this.error = null

    try {
      await api.run(api.applyEdits(this.session.id, this.selected, edits))
      this.countDown()
      await this.select(this.selected)
    } catch (failure) {
      this.error = String(failure)
      throw failure
    } finally {
      this.busy = false
    }
  }

  async select(table: string) {
    if (!this.session) {
      return
    }

    this.selected = table
    this.busy = true

    try {
      this.rows = await api.run(
        api.tableRows(this.session.id, table, this.rowLimit, 0),
      )
      await this.loadSchema()
    } finally {
      this.busy = false
    }
  }

  async loadSchema() {
    if (!this.session || this.schema.length > 0) {
      return
    }

    this.schema = await api.run(api.schema(this.session.id))
  }

  get chosen() {
    const { start, end } = this.selection
    const picked = end > start ? this.sql.slice(start, end) : this.sql

    return picked.trim()
  }

  async run() {
    if (!this.session || this.chosen === "") {
      return
    }

    this.busy = true
    this.queryError = null

    try {
      const fault = await api.run(api.checkSql(this.chosen, this.dialect))

      if (fault) {
        this.queryError = m.sql_fault({
          line: fault.line + 1,
          column: fault.column + 1,
          text: fault.text.slice(0, 40) || "?",
        })
        this.queryResult = null

        return
      }

      this.queryResult = await api.run(
        api.runQuery(this.session.id, this.chosen),
      )
      this.queryRan = true
    } catch (failure) {
      this.queryError = friendly(String(failure))
      this.queryResult = null
    } finally {
      this.busy = false
    }
  }

  clearQuery() {
    this.sql = ""
    this.queryResult = null
    this.queryError = null
    this.queryRan = false
    this.openQuery = null
    this.autosaved = false
  }

  loadSaved(id: string) {
    const entry = this.saved.find(row => row.id === id)

    if (!entry) {
      return
    }

    this.sql = entry.sql
    this.openQuery = entry.id
    this.autosaved = false
  }

  async keep() {
    if (this.sql.trim() === "") {
      return
    }

    const now = Math.floor(Date.now() / 1000)

    if (this.openQuery) {
      await local
        .update(savedQuery)
        .set({ name: firstLine(this.sql), sql: this.sql, savedAt: now })
        .where(eq(savedQuery.id, this.openQuery))
    } else {
      const id = crypto.randomUUID()

      await local.insert(savedQuery).values({
        id,
        name: firstLine(this.sql),
        sql: this.sql,
        target: this.session?.label ?? "",
        savedAt: now,
      })

      this.openQuery = id
    }

    await this.reloadSaved()
  }

  async drop(id: string) {
    await local.delete(savedQuery).where(eq(savedQuery.id, id))

    if (this.openQuery === id) {
      this.openQuery = null
    }

    await this.reloadSaved()
  }

  async forgetRecent(url: string) {
    await local.delete(recent).where(eq(recent.url, url))
    await api.run(api.forgetLogin(url))
    await this.reloadRecents()
  }
}

function firstLine(sql: string) {
  const line = sql.trim().split("\n")[0]

  return line.length > 60 ? `${line.slice(0, 57)}...` : line
}

export const workspace = new Workspace()
