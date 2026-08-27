<script lang="ts">
  import { fade, scale } from "svelte/transition"

  import { Icon, pop, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  let writes = $derived(workspace.writes)
  let pending = $derived(writes.pending)
</script>

{#if pending}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    transition:fade={veil()}
    onclick={() => writes.settle(false)}
    class="fixed inset-0 z-60 grid place-items-center bg-base-300/45 p-6"
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      transition:scale={pop()}
      onclick={event => event.stopPropagation()}
      class="flex max-h-full w-full max-w-2xl flex-col rounded-box floating lift"
    >
      <header class="flex items-center gap-2 px-4 pt-3 pb-2">
        <Icon icon="lucide:file-pen-line" class="size-4 text-warning" />

        <h2 class="flex-1 text-sm font-medium">
          {m.preview_title({ count: pending.statements.length })}
        </h2>

        {#if writes.manual}
          <span class="rounded-field bg-warning/15 px-2 py-0.5 text-xs text-warning">
            {m.tx_manual()}
          </span>
        {/if}
      </header>

      <div class="min-h-0 flex-1 overflow-y-auto px-4">
        <pre
          class="rounded-field bg-base-200 p-3 text-xs whitespace-pre-wrap
            select-text">{pending.statements.join(";\n")};</pre>
      </div>

      <footer class="flex items-center gap-2 px-4 pt-3 pb-3">
        <p class="flex-1 text-xs text-base-content/45">
          {writes.manual ? m.preview_manual_hint() : m.preview_hint()}
        </p>

        <button
          type="button"
          onclick={() => writes.settle(false)}
          class="rounded-field bg-base-200 px-3 py-1.5 text-xs hover:bg-base-300"
        >
          {m.discard()}
        </button>

        <button
          type="button"
          onclick={() => writes.settle(true)}
          class="rounded-field bg-primary px-3 py-1.5 text-xs text-primary-content"
        >
          {m.preview_run()}
        </button>
      </footer>
    </div>
  </div>
{/if}
