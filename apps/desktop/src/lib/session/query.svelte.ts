import { and, desc, eq } from "drizzle-orm"
import type { Advice } from "$lib/ai/advise"
import { local } from "$lib/db/client"
import { queryRun, savedQuery } from "$lib/db/schema"
import * as m from "$lib/paraglide/messages"
import type {
  Plan,
  Provider,
  QueryResult,
  SessionHandle,
  TableSchema,
} from "$lib/types"

import * as api from "./commands"
import { friendly } from "./errors"

export type QueryHost = {
  session: () => SessionHandle | null
  dialect: () => string
  provider: () => Provider | null
  schema: () => Promise<TableSchema[]>
}

function firstLine(sql: string) {
  return sql.trim().split("\n")[0].slice(0, 60) || "query"
}

export class Query {
  sql = $state("")
  selection = $state({ start: 0, end: 0 })

  result = $state<QueryResult | null>(null)
  error = $state<string | null>(null)
  ran = $state(false)
  busy = $state(false)
  spot = $state(false)

  plan = $state<Plan | null>(null)
  analyzed = $state(false)
  advice = $state<Advice | null>(null)
  advising = $state(false)

  history = $state<(typeof queryRun.$inferSelect)[]>([])

  saved = $state<(typeof savedQuery.$inferSelect)[]>([])
  open = $state<string | null>(null)
  autosaved = $state(false)

  private host: QueryHost

  constructor(host: QueryHost) {
    this.host = host
  }

  get chosen() {
    const { start, end } = this.selection
    const picked = end > start ? this.sql.slice(start, end) : this.sql

    return picked.trim()
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
    this.ran = false
    this.open = null
    this.autosaved = false
  }

  async run() {
    const session = this.host.session()

    if (!session || this.chosen === "") {
      return
    }

    const sql = this.chosen
    const started = Date.now()

    this.busy = true
    this.error = null
    this.plan = null

    try {
      const fault = await api.run(
        api.checkSql(this.chosen, this.host.dialect()),
      )

      if (fault) {
        this.error = m.sql_fault({
          line: fault.line + 1,
          column: fault.column + 1,
          text: fault.text.slice(0, 40) || "?",
        })
        this.result = null

        return
      }

      this.result = await api.run(api.runQuery(session.id, sql))
      this.ran = true
      await this.note(sql, session.label, true, Date.now() - started)
    } catch (failure) {
      this.error = friendly(String(failure))
      this.result = null
      await this.note(sql, session.label, false, Date.now() - started)
    } finally {
      this.busy = false
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

    await this.reloadHistory()
  }

  async reloadHistory() {
    this.history = await local
      .select()
      .from(queryRun)
      .orderBy(desc(queryRun.ranAt))
      .limit(100)
  }

  async forgetHistory() {
    await local.delete(queryRun)
    this.history = []
  }

  async explain(analyze: boolean) {
    const session = this.host.session()

    if (!session || this.chosen === "" || this.busy) {
      return
    }

    this.busy = true
    this.error = null

    try {
      this.plan = await api.run(
        api.explainQuery(session.id, this.chosen, analyze),
      )
      this.analyzed = analyze
      this.advice = null
    } catch (failure) {
      this.error = friendly(String(failure))
      this.plan = null
    } finally {
      this.busy = false
    }
  }

  // the model reads the real plan rather than guessing from the sql alone
  async advise() {
    const provider = this.host.provider()
    const plan = this.plan

    if (!provider || !plan || this.advising) {
      return
    }

    this.advising = true
    this.error = null

    try {
      const [{ diagnose }, schema] = await Promise.all([
        import("$lib/ai/advise"),
        this.host.schema(),
      ])

      this.advice = await diagnose(provider, this.chosen, plan, schema)
    } catch (failure) {
      this.error = friendly(String(failure))
    } finally {
      this.advising = false
    }
  }

  async reload() {
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
        target: this.host.session()?.label ?? "",
        savedAt: now,
      })

      this.open = id
    }

    await this.reload()
  }

  async drop(id: string) {
    await local.delete(savedQuery).where(eq(savedQuery.id, id))

    if (this.open === id) {
      this.open = null
    }

    await this.reload()
  }
}
