<script lang="ts">
  import type { NodeProps } from "@xyflow/svelte"

  import { Icon } from "../icons"
  import { board } from "./board.svelte"

  let { data }: NodeProps = $props()

  let id = $derived(data.id as string)
  let name = $derived(data.name as string)
  let count = $derived(data.count as number)

  let editing = $state(false)
  let draft = $state("")

  function begin() {
    draft = name
    editing = true
  }

  function commit() {
    editing = false

    if (draft.trim() !== "" && draft !== name) {
      board.rename?.(id, draft.trim())
    }
  }
</script>

<div
  class="h-full w-full rounded-box border border-primary/25 bg-primary/5"
>
  <header class="flex h-8 items-center gap-2 px-3">
    {#if editing}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        autofocus
        bind:value={draft}
        onblur={commit}
        onkeydown={event => {
          if (event.key === "Enter") {
            event.preventDefault()
            commit()
          }

          if (event.key === "Escape") {
            event.preventDefault()
            editing = false
          }
        }}
        class="nodrag min-w-0 flex-1 bg-transparent text-xs font-medium
          text-primary outline-none select-text"
      />
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span
        ondblclick={begin}
        class="min-w-0 flex-1 truncate text-xs font-medium text-primary"
      >
        {name}
      </span>

      <span class="text-xs text-primary/50">{count}</span>

      <button
        type="button"
        aria-label={name}
        onclick={begin}
        class="nodrag rounded-selector p-1 text-primary/50 hover:text-primary"
      >
        <Icon icon="lucide:pencil" class="size-3" />
      </button>

      <button
        type="button"
        aria-label={name}
        onclick={() => board.ungroup?.(id)}
        class="nodrag rounded-selector p-1 text-primary/50 hover:text-error"
      >
        <Icon icon="lucide:ungroup" class="size-3" />
      </button>
    {/if}
  </header>
</div>
