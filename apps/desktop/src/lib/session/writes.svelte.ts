import type { CellEdit, SessionHandle } from "$lib/types"

import * as api from "./commands"

export type WritesHost = {
  session: () => SessionHandle | null
  remember: (key: string, value: string) => Promise<void>
  refresh: () => Promise<void>
}

type Pending = {
  table: string
  edits: CellEdit[]
  statements: string[]
  resolve: (approved: boolean) => void
}

export class Writes {
  manual = $state(false)
  preview = $state(true)
  open = $state(false)
  busy = $state(false)
  error = $state<string | null>(null)

  pending = $state<Pending | null>(null)

  private host: WritesHost

  constructor(host: WritesHost) {
    this.host = host
  }

  get available() {
    return this.host.session()?.transactional ?? false
  }

  load(settings: Map<string, string>) {
    this.preview = settings.get("previewWrites") !== "off"
  }

  async setPreview(on: boolean) {
    this.preview = on
    await this.host.remember("previewWrites", on ? "on" : "off")
  }

  async setManual(on: boolean) {
    const session = this.host.session()

    if (!session || !this.available) {
      return
    }

    await api.run(api.setManual(session.id, on))

    this.manual = on
    this.open = false

    if (!on) {
      await this.host.refresh()
    }
  }

  // asks the backend for the exact statements, so the approval is not an
  // approximation of what will run
  async confirm(table: string, edits: CellEdit[]) {
    const session = this.host.session()

    if (!session || !this.preview) {
      return true
    }

    const statements = await api.run(api.pendingEdits(session.id, table, edits))

    if (statements.length === 0) {
      return true
    }

    return new Promise<boolean>(resolve => {
      this.pending = { table, edits, statements, resolve }
    })
  }

  settle(approved: boolean) {
    this.pending?.resolve(approved)
    this.pending = null
  }

  noteWrite() {
    if (this.manual) {
      this.open = true
    }
  }

  async end(commit: boolean) {
    const session = this.host.session()

    if (!session || this.busy) {
      return null
    }

    this.busy = true
    this.error = null

    try {
      const had = await api.run(api.endTransaction(session.id, commit))

      this.open = false

      if (had) {
        await this.host.refresh()
      }

      return had
    } catch (failure) {
      this.error = String(failure)

      return null
    } finally {
      this.busy = false
    }
  }

  reset() {
    this.open = false
    this.manual = false
    this.pending?.resolve(false)
    this.pending = null
    this.error = null
  }
}
