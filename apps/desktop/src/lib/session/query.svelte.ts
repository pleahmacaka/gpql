import { and, desc, eq } from "drizzle-orm"
import type { Advice } from "$lib/ai/advise"
import { local } from "$lib/db/client"
import { queryRun, savedQuery } from "$lib/db/schema"
import * as m from "$lib/paraglide/messages"
import { bury } from "$lib/sync/client"
import type {
  Plan,
  Provider,
  QueryResult,
  SessionHandle,
  TableSchema,
} from "$lib/types"

import * as api from "./commands"
import { friendly } from "./errors"
import { unitColumn } from "./tokens"

export type QueryHost = {
  session: () => SessionHandle | null
  target: () => string
  dialect: () => string
  provider: () => Provider | null
  schema: () => Promise<TableSchema[]>
  catalogChanged: () => void
  wrote: () => void
}

// mirrors reads_only in engines/db.rs, which decides when a manual transaction begins
const READS = [
  "select",
  "show",
  "describe",
  "desc",
  "explain",
  "with",
  "pragma",
  "from",
  "match",
  "return",
  "unwind",
  "values",
  "table",
  "ls",
  "list",
  "get",
  "presign",
]

export function readsOnly(sql: string) {
  const head = /^[a-z]*/i.exec(sql.trimStart())?.[0] ?? ""

  return READS.includes(head.toLowerCase())
}

type Fault = { line: number; column: number; text: string }

function firstLine(sql: string) {
  return sql.trim().split("\n")[0].slice(0, 60) || "query"
}

export class Query {
  sql = $state("")
  selection = $state({ start: 0, end: 0 })

  result = $state<QueryResult | null>(null)
  error = $state<string | null>(null)
  millis = $state<number | null>(null)
  composeError = $state("")
  ran = $state(false)
  busy = $state(false)
  spot = $state(false)

  plan = $state<Plan | null>(null)
  analyzed = $state(false)
  advice = $state<Advice | null>(null)
  advising = $state(false)

  history = $state<(typeof queryRun.$inferSelect)[]>([])
  historyLoading = $state(false)

  saved = $state<(typeof savedQuery.$inferSelect)[]>([])
  savedLoading = $state(false)
  open = $state<string | null>(null)
  autosaved = $state(false)

  private host: QueryHost
  private ticket = 0
  private stopping = false
  private inflight: Promise<unknown> = Promise.resolve()
  private forced = ""
  private checked = $state<{ sql: string; fault: Fault } | null>(null)

  constructor(host: QueryHost) {
    this.host = host
  }

  get chosen() {
    const { start, end } = this.selection
    const picked = end > start ? this.sql.slice(start, end) : this.sql

    return picked.trim()
  }

  get fault() {
    const checked = this.checked

    return checked?.sql === this.chosen ? checked.fault : null
  }

  clear() {
    this.sql = ""
    this.reset()
  }

  reset() {
    this.plan = null
    this.advice = null
    this.result = null
    this.error = null
    this.millis = null
    this.composeError = ""
    this.checked = null
    this.ran = false
    this.open = null
    this.autosaved = false
  }

  // tree-sitter misreads some dialect extras, so a repeated run goes through
  private async faultIn(sql: string): Promise<Fault | null> {
    if (sql === this.forced) {
      return null
    }

    const found = await api
      .run(api.checkSql(sql, this.host.dialect()))
      .catch(() => null)

    if (!found) {
      return null
    }

    this.forced = sql

    const line = sql.split("\n")[found.line] ?? ""

    return {
      line: found.line + 1,
      column: unitColumn(line, found.column) + 1,
      text: found.text.slice(0, 40) || "?",
    }
  }

  async run() {
    const session = this.host.session()
    const sql = this.chosen

    if (!session || sql === "" || this.busy) {
      return
    }

    const ticket = ++this.ticket
    const target = this.host.target()
    let started = Date.now()

    this.busy = true
    this.error = null
    this.plan = null
    this.checked = null

    try {
      const fault = await this.faultIn(sql)

      if (ticket !== this.ticket) {
        return
      }

      if (fault) {
        this.checked = { sql, fault }

        return
      }

      const writes = !readsOnly(sql) && !session.readOnly
      let result: QueryResult

      started = Date.now()

      try {
        const sent = api.run(api.runQuery(session.id, sql))

        this.inflight = sent.catch(() => undefined)
        result = await sent
      } finally {
        if (writes) {
          this.host.wrote()
        }
      }

      const millis = Date.now() - started

      if (/\b(create|drop|alter|truncate|rename)\b/i.test(sql)) {
        this.host.catalogChanged()
      }

      if (ticket === this.ticket) {
        this.result = result
        this.millis = millis
        this.ran = true
      }

      await this.note(sql, target, true, millis)
    } catch (failure) {
      if (ticket === this.ticket) {
        this.error = friendly(String(failure))
        this.result = null
        this.millis = null
      }

      await this.note(sql, target, false, Date.now() - started)
    } finally {
      if (ticket === this.ticket) {
        this.busy = false
      }
    }
  }

  // busy holds until the stopped statement ends, so its cancel misses the next
  async stop() {
    const session = this.host.session()

    if (!this.busy || this.stopping) {
      return
    }

    this.ticket++
    this.stopping = true
    this.error = m.query_stopped()

    try {
      if (session) {
        await api.run(api.cancelQuery(session.id))
        await this.inflight
      }
    } catch (failure) {
      this.error = friendly(String(failure))
    } finally {
      this.stopping = false
      this.busy = false
    }
  }

  async compose(write: () => Promise<string>) {
    if (this.busy) {
      return
    }

    const ticket = ++this.ticket

    this.busy = true
    this.composeError = ""

    try {
      const sql = await write()

      if (ticket === this.ticket) {
        await this.replace(sql)
      }
    } catch (failure) {
      if (ticket === this.ticket) {
        this.composeError = friendly(String(failure))
      }
    } finally {
      if (ticket === this.ticket) {
        this.busy = false
      }
    }
  }

  // keeping only the newest run of an identical statement stops the list
  // filling with the same query typed twice
  private async note(sql: string, target: string, ok: boolean, millis: number) {
    await local
      .delete(queryRun)
      .where(and(eq(queryRun.sql, sql), eq(queryRun.target, target)))

    await local.insert(queryRun).values({
      id: crypto.randomUUID(),
      sql,
      target,
      ok,
      millis,
      ranAt: Math.floor(Date.now() / 1000),
    })

    await this.readHistory()
  }

  async reloadHistory() {
    this.historyLoading = true

    try {
      await this.readHistory()
    } finally {
      this.historyLoading = false
    }
  }

  private async readHistory() {
    this.history = await local
      .select()
      .from(queryRun)
      .where(eq(queryRun.target, this.host.target()))
      .orderBy(desc(queryRun.ranAt))
      .limit(100)
  }

  async forgetHistory() {
    await local.delete(queryRun).where(eq(queryRun.target, this.host.target()))
    this.history = []
  }

  async explain(analyze: boolean) {
    const session = this.host.session()
    const sql = this.chosen

    if (!session || sql === "" || this.busy) {
      return
    }

    const ticket = ++this.ticket

    this.busy = true
    this.error = null

    try {
      const sent = api.run(api.explainQuery(session.id, sql, analyze))

      this.inflight = sent.catch(() => undefined)

      const plan = await sent

      if (ticket === this.ticket) {
        this.plan = plan
        this.analyzed = analyze
        this.advice = null
      }
    } catch (failure) {
      if (ticket === this.ticket) {
        this.error = friendly(String(failure))
        this.plan = null
      }
    } finally {
      if (ticket === this.ticket) {
        this.busy = false
      }
    }
  }

  // the model reads the real plan rather than guessing from the sql alone
  async advise() {
    const provider = this.host.provider()
    const plan = this.plan
    const sql = this.chosen

    if (!provider || !plan || this.advising) {
      return
    }

    this.advising = true
    this.composeError = ""

    try {
      const [{ diagnose }, schema] = await Promise.all([
        import("$lib/ai/advise"),
        this.host.schema(),
      ])

      this.advice = await diagnose(provider, sql, plan, schema)
    } catch (failure) {
      this.composeError = friendly(String(failure))
    } finally {
      this.advising = false
    }
  }

  async reload() {
    this.savedLoading = true

    try {
      await this.readSaved()
    } finally {
      this.savedLoading = false
    }
  }

  private async readSaved() {
    this.saved = await local
      .select()
      .from(savedQuery)
      .orderBy(desc(savedQuery.savedAt))
  }

  load(id: string) {
    const entry = this.saved.find(row => row.id === id)

    if (!entry) {
      return
    }

    this.sql = entry.sql
    this.selection = { start: 0, end: 0 }
    this.open = entry.id
    this.autosaved = false
  }

  // the buffer is saved first so a query written for the user never costs theirs
  async replace(sql: string) {
    const stored = this.saved.find(row => row.id === this.open)?.sql

    if (this.sql.trim() !== "" && this.sql !== sql && this.sql !== stored) {
      await this.keep()
    }

    this.sql = sql
    this.selection = { start: 0, end: 0 }
    this.open = null
    this.autosaved = false
  }

  async keep() {
    if (this.sql.trim() === "") {
      return
    }

    const now = Math.floor(Date.now() / 1000)

    if (this.open) {
      await local
        .update(savedQuery)
        .set({ name: firstLine(this.sql), sql: this.sql, savedAt: now })
        .where(eq(savedQuery.id, this.open))
    } else {
      const id = crypto.randomUUID()

      await local.insert(savedQuery).values({
        id,
        name: firstLine(this.sql),
        sql: this.sql,
        target: this.host.target(),
        savedAt: now,
      })

      this.open = id
    }

    await this.readSaved()
  }

  async drop(id: string) {
    await local.delete(savedQuery).where(eq(savedQuery.id, id))
    await bury("query", id)

    if (this.open === id) {
      this.open = null
    }

    await this.readSaved()
  }
}
