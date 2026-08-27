<script lang="ts">
  import { fade } from "svelte/transition"

  import { Icon, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  let writes = $derived(workspace.writes)
</script>

{#if writes.open}
  <div
    transition:fade={veil()}
    class="flex items-center gap-2 rounded-field bg-warning/15 px-3 py-1.5
      text-xs text-warning"
  >
    <Icon icon="lucide:git-commit-horizontal" class="size-3.5 shrink-0" />

    <span class="flex-1">{writes.error ?? m.tx_open()}</span>

    <button
      type="button"
      disabled={writes.busy}
      onclick={() => writes.end(false)}
      class="rounded-selector px-2 py-0.5 hover:bg-warning/20
        disabled:opacity-40"
    >
      {m.tx_rollback()}
    </button>

    <button
      type="button"
      disabled={writes.busy}
      onclick={() => writes.end(true)}
      class="rounded-selector bg-warning px-2 py-0.5 text-warning-content
        disabled:opacity-40"
    >
      {m.tx_commit()}
    </button>
  </div>
{/if}
