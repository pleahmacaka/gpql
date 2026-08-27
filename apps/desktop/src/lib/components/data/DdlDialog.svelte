<script lang="ts">
  import { fade, scale } from "svelte/transition"

  import { Icon, pop, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  let shown = $derived(workspace.ddl)
</script>

{#if shown}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    transition:fade={veil()}
    onclick={() => (workspace.ddl = null)}
    class="fixed inset-0 z-60 grid place-items-center bg-base-300/45 p-6"
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      transition:scale={pop()}
      onclick={event => event.stopPropagation()}
      class="flex max-h-full w-full max-w-3xl flex-col rounded-box floating lift"
    >
      <header class="flex items-center gap-2 px-4 pt-3 pb-2">
        <Icon icon="lucide:file-code-2" class="size-4 text-base-content/40" />

        <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{shown.name}</h2>

        <button
          type="button"
          onclick={() => navigator.clipboard.writeText(shown.text)}
          class="rounded-field bg-base-200 px-2 py-1 text-xs hover:bg-base-300"
        >
          {m.menu_copy()}
        </button>

        <button
          type="button"
          aria-label={m.close()}
          onclick={() => (workspace.ddl = null)}
          class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
        >
          <Icon icon="lucide:x" class="size-4" />
        </button>
      </header>

      <div class="min-h-0 flex-1 overflow-auto px-4 pb-4">
        {#if shown.text === ""}
          <p class="py-6 text-center text-sm text-base-content/45">
            <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
          </p>
        {:else}
          <pre
            class="rounded-field bg-base-200 p-3 text-xs whitespace-pre
              select-text">{shown.text}</pre>
        {/if}
      </div>
    </div>
  </div>
{/if}
