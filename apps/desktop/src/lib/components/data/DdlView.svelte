<script lang="ts">
  import { fade } from "svelte/transition"

  import { Icon, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { highlightSql, run } from "$lib/session/commands"
  import { splitTokens } from "$lib/session/tokens"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { ObjectKind, SqlToken } from "$lib/types"

  const ICONS: Record<ObjectKind, string> = {
    view: "lucide:eye",
    index: "lucide:list-tree",
    sequence: "lucide:hash",
    routine: "lucide:square-function",
    trigger: "lucide:zap",
    type: "lucide:shapes",
  }

  const KINDS: Record<ObjectKind, () => string> = {
    view: m.kind_views,
    index: m.kind_indexes,
    sequence: m.kind_sequences,
    routine: m.kind_routines,
    trigger: m.kind_triggers,
    type: m.kind_types,
  }

  let shown = $derived(workspace.ddl)
  let tokens = $state<SqlToken[]>([])

  $effect(() => {
    const text = shown?.text ?? ""
    const dialect = workspace.dialect

    if (!text) {
      tokens = []
      return
    }

    let live = true

    run(highlightSql(text, dialect))
      .then(found => {
        if (live) {
          tokens = found
        }
      })
      .catch(() => {
        if (live) {
          tokens = []
        }
      })

    return () => {
      live = false
    }
  })

  let pieces = $derived(splitTokens(shown?.text ?? "", tokens))
</script>

{#if shown}
  <header class="flex items-center gap-2 px-4 pt-2 pb-1">
    <Icon
      icon={shown.kind ? ICONS[shown.kind] : "lucide:file-code-2"}
      class="size-4 shrink-0 text-base-content/40"
    />

    <h2 class="min-w-0 truncate text-sm font-medium">{shown.name}</h2>

    {#if shown.kind}
      <span class="text-xs text-base-content/45">{KINDS[shown.kind]()}</span>
    {/if}

    <span class="flex-1"></span>

    <button
      type="button"
      onclick={() => navigator.clipboard.writeText(shown.text)}
      class="rounded-selector bg-base-200 px-2 py-1 text-xs hover:bg-base-300"
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

  <div
    class="min-h-0 flex-1 overflow-y-auto px-4 pt-1 pb-4"
    style:scrollbar-gutter="stable"
  >
    {#if shown.text === ""}
      <p
        in:fade|local={veil()}
        class="py-6 text-center text-sm text-base-content/45"
      >
        <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
      </p>
    {:else}
      <pre
        in:fade|local={veil()}
        class="select-text font-mono text-xs leading-6 whitespace-pre-wrap"
      >{#each pieces as piece, index (index)}<span
            class="tok-{piece.kind}">{piece.text}</span
          >{/each}</pre>
    {/if}
  </div>
{/if}
