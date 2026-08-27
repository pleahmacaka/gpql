import { desc, eq } from "drizzle-orm"

import { local } from "$lib/db/client"
import { chatLog } from "$lib/db/schema"
import { friendly } from "$lib/session/errors"
import type { Provider, TableInfo, TableSchema } from "$lib/types"

import type { PilotMove } from "./pilot"

export type ChatTurn = { id: string; role: "you" | "agent"; text: string }

export type Branch = { at: number; threads: ChatTurn[][]; pick: number }

export type Dock = "off" | "panel" | "orb"

export type ChatHost = {
  provider: () => Provider | null
  context: () => Promise<{ schema: TableSchema[]; tables: TableInfo[] }>
  steer: (move: PilotMove) => Promise<void>
}

type Stored = { turns: ChatTurn[]; branch: Branch | null }

function read(raw: string): Stored {
  const parsed: unknown = JSON.parse(raw)

  if (Array.isArray(parsed)) {
    return { turns: parsed as ChatTurn[], branch: null }
  }

  const held = parsed as Partial<Stored>

  return { turns: held.turns ?? [], branch: held.branch ?? null }
}

function turn(role: ChatTurn["role"], text: string): ChatTurn {
  return { id: crypto.randomUUID(), role, text }
}

export class Chat {
  dock = $state<Dock>("off")
  side = $state<"left" | "center" | "right">("right")

  turns = $state<ChatTurn[]>([])
  branch = $state<Branch | null>(null)
  busy = $state(false)
  error = $state("")

  id = $state<string | null>(null)
  title = $state("")
  saved = $state<(typeof chatLog.$inferSelect)[]>([])

  private host: ChatHost

  constructor(host: ChatHost) {
    this.host = host
  }

  show(dock: Dock) {
    this.dock = this.dock === dock ? "off" : dock
  }

  async reload() {
    this.saved = await local
      .select()
      .from(chatLog)
      .orderBy(desc(chatLog.savedAt))
      .limit(50)
  }

  async keep() {
    if (this.turns.length === 0) {
      return
    }

    this.id ??= crypto.randomUUID()

    const turns = JSON.stringify({
      turns: $state.snapshot(this.turns),
      branch: $state.snapshot(this.branch),
    })
    const now = Math.floor(Date.now() / 1000)

    await local
      .insert(chatLog)
      .values({ id: this.id, title: this.title, turns, savedAt: now })
      .onConflictDoUpdate({
        target: chatLog.id,
        set: { title: this.title, turns, savedAt: now },
      })
    await this.reload()
  }

  start() {
    this.turns = []
    this.branch = null
    this.id = null
    this.title = ""
    this.error = ""
  }

  open(id: string) {
    const found = this.saved.find(entry => entry.id === id)

    if (!found) {
      return
    }

    const held = read(found.turns)

    this.id = found.id
    this.title = found.title
    this.turns = held.turns
    this.branch = held.branch
    this.error = ""
  }

  async drop(id: string) {
    await local.delete(chatLog).where(eq(chatLog.id, id))

    if (this.id === id) {
      this.start()
    }

    await this.reload()
  }

  async deleteTurn(index: number) {
    this.turns = this.turns.filter((_, at) => at !== index)

    if (this.branch && index <= this.branch.at) {
      this.branch = null
    }

    await this.keep()
  }

  async editTurn(index: number, text: string) {
    const snapshot = $state.snapshot(this.turns) as ChatTurn[]

    if (this.branch?.at === index) {
      this.branch.threads[this.branch.pick] = snapshot
      this.branch.threads.push([])
      this.branch.pick = this.branch.threads.length - 1
    } else {
      this.branch = { at: index, threads: [snapshot, []], pick: 1 }
    }

    this.turns = snapshot.slice(0, index)
    await this.send(text)
  }

  async pickThread(pick: number) {
    const branch = this.branch

    if (!branch || pick < 0 || pick >= branch.threads.length) {
      return
    }

    branch.threads[branch.pick] = $state.snapshot(this.turns) as ChatTurn[]
    branch.pick = pick
    this.turns = branch.threads[pick]
    await this.keep()
  }

  private async entitle() {
    const provider = this.host.provider()

    if (this.title !== "" || !provider) {
      return
    }

    try {
      const { entitle } = await import("./title")

      this.title = await entitle(provider, this.turns)
      await this.keep()
    } catch (failure) {
      console.warn("could not name the conversation", failure)
    }
  }

  async send(prompt: string) {
    const text = prompt.trim()

    if (text === "" || this.busy) {
      return
    }

    if (text === "./new" || text === "./clear") {
      this.start()

      return
    }

    const provider = this.host.provider()

    if (!provider) {
      this.error = friendly("gpql.no_model")

      return
    }

    this.turns = [...this.turns, turn("you", text)]
    this.busy = true
    this.error = ""

    try {
      const { pilot } = await import("./pilot")
      const { schema, tables } = await this.host.context()
      const move = await pilot(provider, this.turns, schema, tables)

      await this.host.steer(move)

      this.turns = [...this.turns, turn("agent", move.note)]

      await this.keep()
      void this.entitle()
    } catch (failure) {
      this.error = friendly(String(failure))
    } finally {
      this.busy = false

      if (this.branch) {
        this.branch.threads[this.branch.pick] = $state.snapshot(
          this.turns,
        ) as ChatTurn[]
      }
    }
  }
}
