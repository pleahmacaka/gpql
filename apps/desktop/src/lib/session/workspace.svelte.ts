import { desc, eq } from "drizzle-orm"

import { local } from "$lib/db/client"
import { ErdDocument } from "$lib/erd/document.svelte"
import { migrate } from "$lib/db/migrate"
import { preference, recent, savedQuery } from "$lib/db/schema"
import type {
  BackendInfo,
  Credential,
  Discovery,
  Mode,
  Provider,
  QueryResult,
  SessionConfig,
  SessionHandle,
  Tab,
  TableInfo,
  TableSchema,
} from "$lib/types"
import * as api from "./commands"
import { friendly } from "./errors"
import { blankConfig } from "./commands"

const PAGE = 500

export const DEFAULT_AGENT = "npx @zed-industries/claude-code-acp"

export type ChatTurn = { role: "you" | "agent"; text: string }

export const schemes = ["system", "light", "dark"] as const

export type Scheme = (typeof schemes)[number]

function readScheme(value: string | undefined): Scheme {
  return schemes.includes(value as Scheme) ? (value as Scheme) : "system"
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
  acrylic = $state(false)
  autoscan = $state(true)
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

  found = $state<Discovery[]>([])
  scanning = $state(false)
  presets = $state<Credential[]>([])
  catalog = $state<BackendInfo[]>([])
  servers = $state<string[]>([])
  agent = $state(false)
  shared = $state("")
  agentCommand = $state("")
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
    this.readOnly = settings.get("readOnly") !== "off"
    this.acrylic = settings.get("acrylic") === "on"
    this.autoscan = settings.get("autoscan") !== "off"
    this.settled = settings.get("settled") === "yes"

    this.agentCommand = settings.get("agent") ?? ""

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
    key: "dark" | "compact" | "readOnly" | "acrylic" | "autoscan",
  ): Promise<void> {
    if (key === "dark") {
      return this.setScheme(this.dark ? "light" : "dark")
    }

    this[key] = !this[key]
    await this.remember(key, this[key] ? "on" : "off")

    if (key === "acrylic") {
      await this.paint()
    }
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

  async ensureAgent() {
    if (this.agent) {
      return
    }

    const line = this.agentCommand.trim() || DEFAULT_AGENT

    await this.startAgent(line)
  }

  async say(prompt: string) {
    const text = prompt.trim()

    if (text === "" || this.chatBusy) {
      return
    }

    this.chat = [...this.chat, { role: "you", text }]
    this.chatBusy = true
    this.chatError = ""

    try {
      await this.ensureAgent()

      const answer = await api.run(api.agentChat(text))

      this.chat = [...this.chat, { role: "agent", text: answer }]
    } catch (failure) {
      this.chatError = friendly(String(failure))
    } finally {
      this.chatBusy = false
    }
  }

  async startAgent(line: string) {
    const [program, ...args] = line.trim().split(/\s+/)

    if (!program) {
      await api.run(api.agentStop())
      this.agent = false
      await this.remember("agent", "")

      return
    }

    await api.run(api.agentStart(program, args))
    this.agent = await api.run(api.agentReady())
    this.agentCommand = line
    await this.remember("agent", line)
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
      this.sql = await api.run(api.askSql(providerId, prompt, this.session.id))
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

      this.session = handle
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

  async resume(url: string, kind = "") {
    if (kind === "erd") {
      await this.startErd(url, true)

      return
    }

    const logins = await api.run(api.savedLogins())
    const match = logins.find(login => login.url === url)

    if (!match) {
      this.error = friendly("gpql.login_gone")

      return
    }

    await this.open({
      ...blankConfig(match.kind),
      ...match,
      readOnly: this.readOnly,
    })
  }

  async startErd(path: string, existing: boolean) {
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
  }

  async close() {
    if (!this.session) {
      return
    }

    await api.run(api.disconnect(this.session.id))
    this.session = null
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

  async select(table: string) {
    if (!this.session) {
      return
    }

    this.selected = table
    this.rows = await api.run(api.tableRows(this.session.id, table, PAGE, 0))
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
  }

  async keep() {
    if (this.sql.trim() === "") {
      return
    }

    await local.insert(savedQuery).values({
      id: crypto.randomUUID(),
      name: firstLine(this.sql),
      sql: this.sql,
      target: this.session?.label ?? "",
      savedAt: Math.floor(Date.now() / 1000),
    })

    await this.reloadSaved()
  }

  async drop(id: string) {
    await local.delete(savedQuery).where(eq(savedQuery.id, id))
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
