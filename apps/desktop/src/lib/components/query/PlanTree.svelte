<script lang="ts">
  import { Icon } from "@gpql/ui"

  import type { PlanNode } from "$lib/types"

  import PlanTree from "./PlanTree.svelte"

  type Props = { node: PlanNode; slowest: number; depth?: number }

  let { node, slowest, depth = 0 }: Props = $props()

  let open = $state(true)

  let weight = $derived(
    node.time !== null && slowest > 0 ? node.time / slowest : 0,
  )

  let hot = $derived(weight > 0.5)
</script>

<div class={[depth > 0 && "ml-4 border-l border-base-content/10"]}>
  <div
    class={[
      "flex min-h-8 items-center gap-2 py-1 pr-3",
      depth > 0 ? "pl-2" : "pl-4",
      hot && "bg-warning/5",
    ]}
  >
    {#if node.children.length > 0}
      <button
        type="button"
        aria-label={node.label}
        aria-expanded={open}
        onclick={() => (open = !open)}
        class="btn btn-square btn-ghost btn-xs shrink-0"
      >
        <Icon
          icon="lucide:chevron-right"
          class={["size-4 transition-transform", open && "rotate-90"]}
        />
      </button>
    {:else}
      <span class="w-6 shrink-0"></span>
    {/if}

    <span class="min-w-0 flex-1 truncate text-sm" title={node.label}>
      {node.label}
    </span>

    <span
      class={[
        "w-24 shrink-0 text-right text-xs text-base-content/70",
        "tabular-nums",
      ]}
    >
      {node.rows !== null ? Math.round(node.rows).toLocaleString() : ""}
    </span>

    <span class="relative w-24 shrink-0 text-right text-xs tabular-nums">
      {#if node.time !== null}
        <span
          aria-hidden="true"
          class={[
            "absolute inset-y-0 right-0",
            hot ? "bg-warning/25" : "bg-base-content/10",
          ]}
          style:width="{weight * 100}%"
        ></span>

        <span class="relative pr-1">{node.time.toFixed(2)} ms</span>
      {:else if node.cost !== null}
        <span class="text-base-content/70">{node.cost.toFixed(0)}</span>
      {/if}
    </span>
  </div>

  {#if node.detail}
    <p
      class={[
        "pr-3 pb-1 text-xs whitespace-pre-line text-base-content/70",
        depth > 0 ? "pl-10" : "pl-12",
      ]}
    >
      {node.detail}
    </p>
  {/if}

  {#if open && node.children.length > 0}
    {#each node.children as child, index (index)}
      <PlanTree node={child} {slowest} depth={depth + 1} />
    {/each}
  {/if}
</div>
