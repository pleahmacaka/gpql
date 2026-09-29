import { open } from "@tauri-apps/plugin-dialog"

import { blankConfig } from "$lib/session/commands"
import { friendly } from "$lib/session/errors"
import { workspace } from "$lib/session/workspace.svelte"

export type View = "home" | "new" | "quick" | "recent"

export const FILES: Record<string, { extensions: string[]; name: string }> = {
  sqlite: { extensions: ["db", "sqlite", "sqlite3"], name: "database.db" },
  duckdb: { extensions: ["duckdb", "ddb"], name: "database.duckdb" },
}

export const DATABASE_FILTERS = [
  {
    name: "Database",
    extensions: Object.values(FILES).flatMap(entry => entry.extensions),
  },
]

export async function openDatabase() {
  try {
    const path = await open({
      multiple: false,
      directory: false,
      filters: DATABASE_FILTERS,
    })

    if (typeof path !== "string") {
      return
    }

    const extension = path.split(".").pop()?.toLowerCase() ?? ""
    const kind = FILES.duckdb.extensions.includes(extension)
      ? "duckdb"
      : "sqlite"

    await workspace.open({ ...blankConfig(kind), path, create: false })
  } catch (failure) {
    workspace.error = friendly(String(failure))
  }
}

class Launcher {
  view = $state<View>("home")

  origin = $state<string | null>(null)

  open(view: View, origin: string | null = null) {
    this.origin = origin
    this.view = view
    workspace.mode = view === "new" || view === "quick" ? view : "recent"

    if (workspace.session && !workspace.connecting) {
      workspace.connecting = true
    }
  }

  back() {
    this.view = "home"
    workspace.mode = "recent"
  }
}

export const launcher = new Launcher()
