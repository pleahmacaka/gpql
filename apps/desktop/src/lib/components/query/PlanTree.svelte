<script lang="ts">
  import { Icon } from "@gpql/ui"

  import type { PlanNode } from "$lib/types"

  import PlanTree from "./PlanTree.svelte"

  type Props = { node: PlanNode; slowest: number; depth?: number }

  let { node, slowest, depth = 0 }: Props = $props()

  let open = $state(true)

  // a share of the slowest step, so the eye lands on what actually costs time
  let weight = $derived(
    node.time !== null && slowest > 0 ? node.time / slowest : 0,
  )
</script>

<div class="text-xs" style:padding-left="{depth > 0 ? 1 : 0}rem">
  <div class="flex items-center gap-1.5 py-0.5">
    {#if node.children.length > 0}
      <button
        type="button"
        aria-label={node.label}
        aria-expanded={open}
        onclick={() => (open = !open)}
        class="rounded-selector text-base-content/35 hover:text-base-content"
      >
        <Icon
          icon={open ? "lucide:chevron-down" : "lucide:chevron-right"}
          class="size-3"
        />
      </button>
    {:else}
      <span class="w-3 shrink-0"></span>
    {/if}

    <span
      class="min-w-0 flex-1 truncate {weight > 0.5
        ? 'text-warning'
        : 'text-base-content/80'}"
    >
      {node.label}
    </span>

    {#if node.rows !== null}
      <span class="shrink-0 text-base-content/40">
        {Math.round(node.rows).toLocaleString()} rows
      </span>
    {/if}

    {#if node.time !== null}
      <span
        class="w-16 shrink-0 text-right {weight > 0.5
          ? 'text-warning'
          : 'text-base-content/40'}"
      >
        {node.time.toFixed(2)} ms
      </span>
    {:else if node.cost !== null}
      <span class="w-16 shrink-0 text-right text-base-content/40">
        {node.cost.toFixed(0)}
      </span>
    {/if}
  </div>

  {#if node.detail}
    <p
      class="pb-1 pl-4 text-base-content/40 whitespace-pre-line"
    >
      {node.detail}
    </p>
  {/if}

  {#if open}
    {#each node.children as child, index (index)}
      <PlanTree node={child} {slowest} depth={depth + 1} />
    {/each}
  {/if}
</div>
