<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog"

  import * as m from "$lib/paraglide/messages"
  import { ERD_EXTENSION } from "$lib/erd/document.svelte"
  import { workspace } from "$lib/session/workspace.svelte"

  import { Icon } from "@gpql/ui"

  const filters = [{ name: "GPQL ERD", extensions: [ERD_EXTENSION] }]

  let failure = $state("")

  async function create() {
    failure = ""

    try {
      const path = await save({
        filters,
        defaultPath: `diagram.${ERD_EXTENSION}`,
      })

      if (path) {
        await workspace.startErd(path, false)
      }
    } catch (problem) {
      failure = String(problem)
    }
  }

  async function pick() {
    failure = ""

    try {
      const path = await open({ multiple: false, directory: false, filters })

      if (typeof path === "string") {
        await workspace.startErd(path, true)
      }
    } catch (problem) {
      failure = String(problem)
    }
  }
</script>

<div class="space-y-2">
  <p class="px-1 pb-1 text-xs text-base-content/45">{m.erd_intro()}</p>

  <button
    type="button"
    onclick={create}
    class="flex w-full items-center gap-3 rounded-field bg-primary/10 px-3 py-3
      text-left transition-colors hover:bg-primary/15"
  >
    <Icon icon="lucide:file-plus-2" class="size-4 text-accent" />

    <span class="flex-1">
      <span class="block text-sm text-primary">{m.erd_new()}</span>
      <span class="block text-xs text-base-content/45">{m.erd_new_hint()}</span>
    </span>
  </button>

  <button
    type="button"
    onclick={pick}
    class="flex w-full items-center gap-3 rounded-field bg-base-200 px-3 py-3
      text-left transition-colors hover:bg-base-300"
  >
    <Icon icon="lucide:folder-open" class="size-4 text-base-content/40" />

    <span class="flex-1">
      <span class="block text-sm">{m.erd_open()}</span>
      <span class="block text-xs text-base-content/45">{m.erd_open_hint()}</span>
    </span>
  </button>

  {#if failure}
    <p class="px-1 pt-1 text-xs text-error">{failure}</p>
  {/if}
</div>
