<script lang="ts">
  import { Dialog, type Ending, Icon } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  import SqlLines from "./SqlLines.svelte"

  let writes = $derived(workspace.writes)
  let pending = $derived(writes.pending)
  let ending = $state<Ending>("cancel")

  function settle(run: boolean) {
    ending = run ? "confirm" : "cancel"
    writes.settle(run)
  }
</script>

{#if pending}
  <Dialog
    label={m.preview_title({ count: pending.statements.length })}
    onclose={() => settle(false)}
    dismiss={m.cancel()}
    size="lg"
    tone="warning"
    {ending}
  >
    <header class="flex items-center gap-3 px-6 pt-6 pb-4">
      <span class="grid size-8 place-items-center bg-warning/15 text-warning">
        <Icon icon="lucide:file-pen-line" class="size-4" />
      </span>

      <div class="min-w-0 flex-1">
        <h2 class="text-base font-semibold tracking-tight">
          {m.preview_title({ count: pending.statements.length })}
        </h2>

        <p class="text-xs text-base-content/70">{pending.table}</p>
      </div>

      {#if writes.manual}
        <span class="badge badge-sm badge-soft badge-warning">
          {m.tx_manual()}
        </span>
      {/if}
    </header>

    <div class="min-h-0 flex-1 px-6">
      <div class="max-h-80 overflow-y-auto bg-base-200 py-3 hairline">
        <SqlLines
          wrap
          text={`${pending.statements.join(";\n")};`}
          class="bg-base-200"
        />
      </div>
    </div>

    <footer class="flex items-center gap-2 px-6 pt-4 pb-6">
      <p class="flex-1 text-xs text-base-content/70">
        {writes.manual ? m.preview_manual_hint() : m.preview_hint()}
      </p>

      <button
        type="button"
        onclick={() => settle(false)}
        class="btn btn-ghost btn-sm font-medium"
      >
        {m.cancel()}
      </button>

      <button
        type="button"
        data-autofocus
        onclick={() => settle(true)}
        class="btn btn-warning btn-sm font-medium"
      >
        <Icon icon="lucide:play" class="size-4" />
        {m.preview_run()}
      </button>
    </footer>
  </Dialog>
{/if}
