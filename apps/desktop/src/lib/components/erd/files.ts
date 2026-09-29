import { open } from "@tauri-apps/plugin-dialog"

import { ERD_FILTERS } from "$lib/erd/document.svelte"
import { friendly } from "$lib/session/errors"
import { workspace } from "$lib/session/workspace.svelte"

async function attempt(work: () => Promise<void>) {
  try {
    await work()
  } catch (failure) {
    workspace.error = friendly(String(failure))
  }
}

export const newDiagram = () => attempt(() => workspace.startErd(null, false))

export const openDiagram = () =>
  attempt(async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: ERD_FILTERS,
    })

    if (typeof path === "string") {
      await workspace.startErd(path, true)
    }
  })
