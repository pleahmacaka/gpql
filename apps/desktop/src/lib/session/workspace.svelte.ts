import { board, rem } from "@gpql/ui"
import { asc, desc, eq } from "drizzle-orm"

import { local } from "$lib/db/client"
import { migrate } from "$lib/db/migrate"
import { preference, recent, savedQuery } from "$lib/db/schema"
import { ErdDocument } from "$lib/erd/document.svelte"
import { getLocale, setLocale } from "$lib/paraglide/runtime"
import type {
  BackendInfo,
  Credential,
  Discovery,
  Mode,
  Provider,
  SavedLogin,
  SessionConfig,
  SharedErd,
  Tab,
} from "$lib/types"
import * as api from "./commands"
import { blankConfig } from "./commands"
import { Connection, type ConnectionHost, idle } from "./connection.svelte"
import { foldersOf } from "./connections"
import { friendly } from "./errors"

const PAGE = 1000
const LIMITS = [200, 500, 1000, 5000, 20000]
const WINDOW = 5
const WINDOWS = [0, 1, 5, 15, 30]

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

function hopping(config: SessionConfig) {
  return (config.tunnel?.host ?? "").trim() !== ""
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
    tunnel: login.tunnel ?? blankConfig(login.kind).tunnel,
    readOnly,
  }
}

export class Workspace {
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

  connections = $state<Connection[]>([])
  activeId = $state<string | null>(null)
  asideWidth = $state(256)
  startup = $state<"last" | "recent">("last")

  recents = $state<(typeof recent.$inferSelect)[]>([])
  connectionView = $state<"list" | "grid">("list")
  finding = $state(false)
  notice = $state("")
  ddl = $state<{ name: string; text: string } | null>(null)

  found = $state<Discovery[]>([])
  scanning = $state(false)
  presets = $state<Credential[]>([])
  catalog = $state<BackendInfo[]>([])
  servers = $state<string[]>([])
  shared = $state<SharedErd | null>(null)
  connecting = $state(false)
  adding = $state(false)
  unreachable = $state<Record<string, string>>({})
  dialing = $state<string | null>(null)
  editing = $state<string | null>(null)
  languageServers = $state<Record<string, string>>({})
  providers = $state<Provider[]>([])
  signedIn = $state(false)

  busy = $state(false)
  error = $state<string | null>(null)

  private wiring: ConnectionHost = {
    pageSize: () => this.rowLimit,
    dialect: kind =>
      this.catalog.find(entry => entry.id === kind)?.dialect ?? "sql",
    provider: () => this.model,
    remember: (key, value) => this.remember(key, value),
    steer: async (connection, move) => {
      switch (move.go) {
        case "query":
          this.tab = "query"
          connection.query.sql = move.sql
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

  get browse() {
    return (this.active ?? idle).browse
  }

  get query() {
    return (this.active ?? idle).query
  }

  get writes() {
    return (this.active ?? idle).writes
  }

  get chat() {
    return (this.active ?? idle).chat
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
    this.startup = settings.get("startup") === "recent" ? "recent" : "last"
    this.connectionView =
      settings.get("connectionView") === "grid" ? "grid" : "list"
    this.writes.load(settings)

    const side = settings.get("orbSide")

    if (side === "center" || side === "left") {
      this.chat.side = side
    }

    this.asideWidth = Number(settings.get("asideWidth")) || this.asideWidth

    for (const [key, value] of settings) {
      if (key.startsWith("lsp:")) {
        this.languageServers[key.slice(4)] = value
      }
    }
    this.texture = Number(settings.get("texture") ?? 35)

    await api.run(api.resetSessions())

    await this.paint()

    await Promise.all([
      this.reloadRecents(),
      this.chat.reload(),
      this.query.reload(),
      this.query.reloadHistory(),
      this.reloadCatalog(),
      this.reloadPresets(),
      this.reloadProviders(),
      this.refreshAccount(),
    ])

    const last = this.recents[0]

    if (this.settled && this.startup === "last" && last) {
      void this.resume(last.url, last.kind)
    }
  }

  async setStartup(mode: "last" | "recent") {
    this.startup = mode
    await this.remember("startup", mode)
  }

  async setOrbSide(side: "left" | "center" | "right") {
    this.chat.side = side
    await this.remember("orbSide", side)
  }

  async setAsideWidth(width: number) {
    this.asideWidth = width
    await this.remember("asideWidth", String(width))
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
      for (const entry of this.connections) {
        await api.run(api.setReadOnly(entry.id, this.readOnly))
        entry.handle = { ...entry.handle, readOnly: this.readOnly }
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

    if (this.browse.table) {
      await this.select(this.browse.table)
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
    this.query.saved = []
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

    const answers = await api.run(api.probeRecents(items))
    const found: Record<string, string> = {}

    items.forEach((item, index) => {
      found[item.url] = answers[index] ?? ""
    })

    this.unreachable = found
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

  async setShareOpen(site: string, open: boolean) {
    if (!this.shared) {
      return
    }

    const answer = await api.run(api.shareErd(site, this.shared.id, open))

    this.shared = { ...this.shared, open: answer }
  }

  async reloadCatalog() {
    this.catalog = await api.run(api.backends())
  }

  async reloadPresets() {
    this.presets = await api.run(api.credentials())
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

    this.query.busy = true
    this.query.error = null

    try {
      const provider = this.providers.find(entry => entry.id === providerId)

      if (!provider) {
        throw new Error("gpql.no_model")
      }

      const { writeSql } = await import("$lib/ai/sql")

      this.query.sql = await writeSql(
        provider,
        prompt,
        this.schema,
        this.query.sql,
      )
      this.query.selection = { start: 0, end: 0 }
    } catch (failure) {
      this.query.error = friendly(String(failure))
    } finally {
      this.query.busy = false
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
          tunnelled: hopping(config) ? 1 : 0,
          openedAt: stamp,
        })
        .onConflictDoUpdate({
          target: recent.url,
          set: {
            openedAt: stamp,
            label: handle.label,
            detail: handle.detail,
            tunnelled: hopping(config) ? 1 : 0,
          },
        })

      const stale = this.editing

      if (stale && stale !== api.describe(config)) {
        await local.delete(recent).where(eq(recent.url, stale))
        await api.run(api.forgetLogin(stale))
      }

      const opened = new Connection(handle, this.wiring)

      opened.origin = api.describe(config)
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
      await opened.loadTables()
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
      .values({
        url,
        kind: config.kind,
        label,
        detail,
        tunnelled: hopping(config) ? 1 : 0,
        openedAt: stamp,
      })
      .onConflictDoUpdate({
        target: recent.url,
        set: {
          openedAt: stamp,
          label,
          detail,
          tunnelled: hopping(config) ? 1 : 0,
        },
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

  async close(id = this.activeId) {
    const going = this.connections.find(entry => entry.id === id)

    if (!going) {
      return
    }

    await going.close()

    this.connections = this.connections.filter(entry => entry.id !== going.id)
    this.activeId = this.connections[0]?.id ?? null

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
    if (this.connections.some(entry => entry.id === id)) {
      this.activeId = id
      this.adding = false
      this.tab = "data"
      board.reset()
    }
  }

  async useSchema(name: string) {
    await this.active?.useSchema(name)
  }

  async showDdl(name: string) {
    const session = this.session

    if (!session) {
      return
    }

    this.ddl = { name, text: "" }

    try {
      const text = await api.run(api.tableDdl(session.id, name))

      if (this.ddl) {
        this.ddl = { name, text }
      }
    } catch (failure) {
      this.ddl = { name, text: friendly(String(failure)) }
    }
  }

  resetCatalog() {
    this.notice = ""
    this.ddl = null
    this.active?.forgetCatalog()
    board.reset()
  }

  async loadTables() {
    await this.active?.loadTables()
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

    const [table, key] = target.split(".")

    if (!table || !key) {
      return
    }

    this.tab = "data"
    await this.browse.open(table)
    await this.browse.setFilters({
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
    this.error = null

    try {
      await this.active?.applyEdits(edits)
      this.countDown()
    } catch (failure) {
      this.error = String(failure)
      throw failure
    } finally {
      this.busy = false
    }
  }

  async select(table: string) {
    await this.active?.select(table)
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
    await local.delete(recent).where(eq(recent.url, url))
    await api.run(api.forgetLogin(url))
    await this.reloadRecents()
  }
}

export const workspace = new Workspace()
