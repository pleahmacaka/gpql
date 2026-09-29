<script lang="ts">
  import { fade } from "svelte/transition"

  import { ConfirmDialog, Icon, Marker, tooltip, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { hint } from "$lib/session/errors"
  import { workspace } from "$lib/session/workspace.svelte"

  let writes = $derived(workspace.writes)
  let open = $derived(
    writes.open || (workspace.active?.openTransaction ?? false),
  )
  let confirming = $state(false)
  let advice = $derived(writes.error ? hint(writes.error) : "")

  async function rollback() {
    confirming = false
    await writes.end(false)
  }
</script>

{#if open}
  <div
    transition:fade|local={veil()}
    role="status"
    class="flex min-w-0 shrink items-center gap-2"
  >
    {#if writes.error}
      <span
        use:tooltip={advice ? `${writes.error}\n${advice}` : writes.error}
        class="flex min-w-0 items-center gap-2 text-error"
      >
        <Icon icon="lucide:circle-alert" class="size-4 shrink-0" />
        <span class="truncate select-text">{writes.error}</span>
      </span>
    {:else}
      <Marker
        label={m.tx_open()}
        tone="warning"
        as="span"
        class="min-w-0 truncate"
      />
    {/if}

    <button
      type="button"
      disabled={writes.busy}
      onclick={() => (confirming = true)}
      class="btn btn-ghost btn-sm shrink-0"
    >
      <Icon icon="lucide:undo-2" class="size-4" />
      {m.tx_rollback()}
    </button>

    <button
      type="button"
      disabled={writes.busy}
      onclick={() => writes.end(true)}
      class="btn btn-warning btn-sm shrink-0 font-medium"
    >
      <Icon icon="lucide:git-commit-horizontal" class="size-4" />
      {m.tx_commit()}
    </button>
  </div>
{/if}

{#if confirming}
  <ConfirmDialog
    title={m.tx_rollback_ask()}
    body={m.tx_rollback_hint()}
    confirm={m.tx_rollback()}
    cancel={m.cancel()}
    icon="lucide:undo-2"
    onconfirm={rollback}
    oncancel={() => (confirming = false)}
  />
{/if}
