<script lang="ts">
  import type { NodeProps } from "@xyflow/svelte"

  import { tooltip } from "../controls/tooltip"
  import { Icon } from "../icons"
  import { board, spoken } from "./board.svelte"

  let { data }: NodeProps = $props()

  const words = spoken()

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
    if (!editing) {
      return
    }

    editing = false

    if (draft.trim() !== "" && draft.trim() !== name) {
      board.rename?.(id, draft.trim())
    }
  }

  function keys(event: KeyboardEvent) {
    if (event.isComposing) {
      return
    }

    if (event.key === "Enter") {
      event.preventDefault()
      commit()
    }

    if (event.key === "Escape") {
      event.preventDefault()
      event.stopPropagation()
      editing = false
    }
  }

  function grab(node: HTMLInputElement) {
    node.focus()
    node.select()
  }
</script>

<section aria-label={name} class="relative size-full bg-primary/5">
  <header class="flex h-10 items-center gap-2 px-4">
    <span aria-hidden="true" class="size-2 shrink-0 bg-primary"></span>

    {#if editing}
      <input
        bind:value={draft}
        onblur={commit}
        onkeydown={keys}
        aria-label={words().rename}
        {@attach grab}
        class={[
          "nodrag input input-xs min-w-0 flex-1 bg-base-100 text-xs",
          "font-medium",
        ]}
      />
    {:else}
      <span class="min-w-0 flex-1 truncate text-xs font-medium text-primary">
        {name}
      </span>

      <span class="text-xs text-base-content/70 tabular-nums">{count}</span>

      <button
        type="button"
        aria-label="{words().rename}, {name}"
        use:tooltip={words().rename}
        onclick={begin}
        class="nodrag btn btn-square btn-ghost btn-xs"
      >
        <Icon icon="lucide:pencil" class="size-3" />
      </button>

      <button
        type="button"
        aria-label="{words().ungroup}, {name}"
        use:tooltip={words().ungroup}
        onclick={() => board.ungroup?.(id)}
        class="nodrag btn btn-square btn-ghost btn-xs hover:text-error"
      >
        <Icon icon="lucide:ungroup" class="size-3" />
      </button>
    {/if}
  </header>

  <span
    aria-hidden="true"
    class="hud hud-wide pointer-events-none absolute inset-0"
  ></span>
</section>
