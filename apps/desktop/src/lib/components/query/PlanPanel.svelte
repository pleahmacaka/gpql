<script lang="ts">
  import { Icon } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { PlanNode } from "$lib/types"

  let query = $derived(workspace.query)

  function slowestIn(node: PlanNode): number {
    return node.children.reduce(
      (worst, child) => Math.max(worst, slowestIn(child)),
      node.time ?? 0,
    )
  }

  let slowest = $derived(query.plan?.tree ? slowestIn(query.plan.tree) : 0)
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <header class="flex items-center gap-2 px-4 pb-2">
    <Icon icon="lucide:git-fork" class="size-4 text-base-content/40" />

    <h2 class="flex-1 text-xs text-base-content/45">
      {query.analyzed ? m.plan_measured() : m.plan_estimated()}
    </h2>

    {#if workspace.ai && workspace.model}
      <button
        type="button"
        disabled={query.advising}
        onclick={() => query.advise()}
        class="flex items-center gap-2 rounded-field bg-base-200 px-2 py-1
          text-xs hover:bg-base-300 disabled:opacity-40"
      >
        <Icon
          icon={query.advising
            ? "lucide:loader-circle"
            : "lucide:stethoscope"}
          class="size-4 {query.advising ? 'animate-spin' : ''}"
        />
        {m.plan_diagnose()}
      </button>
    {/if}

    <button
      type="button"
      onclick={() => (query.plan = null)}
      class="rounded-field bg-base-200 px-2 py-1 text-xs hover:bg-base-300"
    >
      {m.close()}
    </button>
  </header>

  {#if query.advice}
    <div class="mx-4 mb-2 rounded-field bg-base-200 p-3 select-text">
      <p class="text-xs">{query.advice.verdict}</p>

      {#each query.advice.steps as step, index (index)}
        <div class="pt-2">
          <p class="text-xs text-base-content/60">· {step.why}</p>

          {#if step.sql}
            <button
              type="button"
              onclick={() => {
                query.sql = step.sql
                query.plan = null
              }}
              class="mt-1 w-full rounded-field bg-base-300 px-2 py-1 text-left
                text-xs whitespace-pre-wrap hover:bg-base-100"
            >
              {step.sql}
            </button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <div class="min-h-0 flex-1 overflow-auto px-4 pb-3 select-text">
    {#if query.plan?.tree}
      {#await import("./PlanTree.svelte") then loaded}
        <loaded.default node={query.plan.tree} {slowest} />
      {/await}
    {:else}
      <pre class="text-xs whitespace-pre-wrap">{query.plan?.text ?? ""}</pre>
    {/if}
  </div>
</div>
