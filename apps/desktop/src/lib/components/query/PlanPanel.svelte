<script lang="ts">
  import { fade } from "svelte/transition"

  import { Icon, Marker, rise, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { PlanNode } from "$lib/types"

  import Code from "./Code.svelte"
  import PlanTree from "./PlanTree.svelte"

  let query = $derived(workspace.query)

  function slowestIn(node: PlanNode): number {
    return node.children.reduce(
      (worst, child) => Math.max(worst, slowestIn(child)),
      node.time ?? 0,
    )
  }

  let slowest = $derived(query.plan?.tree ? slowestIn(query.plan.tree) : 0)

  let timed = $derived(slowest > 0)

  async function use(sql: string) {
    query.plan = null
    await query.replace(sql)
  }
</script>

<div
  class={[
    "flex h-10 shrink-0 items-center gap-2 border-b border-base-content/10",
    "pr-2 pl-3",
  ]}
>
  <Marker
    as="h2"
    label={query.analyzed ? m.plan_measured() : m.plan_estimated()}
    class="min-w-0 flex-1"
  />

  {#if workspace.ai && workspace.model}
    <button
      type="button"
      disabled={query.advising}
      onclick={() => query.advise()}
      class="btn btn-soft btn-sm font-medium"
    >
      <Icon
        icon={query.advising ? "lucide:loader-circle" : "lucide:stethoscope"}
        class={["size-4", query.advising && "animate-spin"]}
      />
      {m.plan_diagnose()}
    </button>
  {/if}

  <button
    type="button"
    aria-label={m.close()}
    onclick={() => (query.plan = null)}
    class="btn btn-square btn-ghost btn-sm"
  >
    <Icon icon="lucide:x" class="size-4" />
  </button>
</div>

<div class="min-h-0 flex-1 overflow-auto" style:scrollbar-gutter="stable">
  {#if query.advising && !query.advice}
    <p
      in:fade={veil()}
      role="status"
      class={[
        "m-3 flex h-12 items-center gap-2 bg-base-200 px-4 text-sm",
        "text-base-content/70 hairline",
      ]}
    >
      <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
      {m.plan_reading()}
    </p>
  {:else if query.composeError}
    <p
      in:fade={veil()}
      role="alert"
      class={[
        "m-3 flex items-start gap-2 bg-error/5 px-4 py-3 text-sm",
        "wrap-anywhere select-text hairline",
      ]}
    >
      <Icon icon="lucide:circle-x" class="mt-1 size-4 shrink-0 text-error" />
      <span class="min-w-0">{query.composeError}</span>
    </p>
  {:else if query.advice}
    <section
      in:rise
      aria-label={m.plan_advice()}
      class="relative m-3 flex flex-col gap-4 bg-base-200 p-4 hairline"
    >
      <span
        aria-hidden="true"
        class="hud hud-small hud-lit pointer-events-none absolute inset-0"
      ></span>

      <div class="flex flex-col gap-2">
        <Marker as="h3" label={m.plan_advice()} />
        <p class="text-sm text-pretty select-text">{query.advice.verdict}</p>
      </div>

      {#if query.advice.steps.length > 0}
        <ol class="flex flex-col gap-4">
          {#each query.advice.steps as step, index (index)}
            <li class="flex gap-3">
              <span class="pt-1 text-xs font-medium text-primary tabular-nums">
                {String(index + 1).padStart(2, "0")}
              </span>

              <div class="flex min-w-0 flex-1 flex-col gap-2">
                <p class="text-sm text-pretty select-text">{step.why}</p>

                {#if step.sql}
                  <div class="flex flex-col gap-2 bg-base-100 p-3 hairline">
                    <Code code={step.sql} class="text-xs leading-5" />

                    <button
                      type="button"
                      onclick={() => use(step.sql)}
                      class={[
                        "btn btn-soft btn-primary btn-xs self-start",
                        "font-medium",
                      ]}
                    >
                      <Icon icon="lucide:corner-down-left" class="size-4" />
                      {m.plan_use()}
                    </button>
                  </div>
                {/if}
              </div>
            </li>
          {/each}
        </ol>
      {/if}
    </section>
  {/if}

  {#if query.plan?.tree}
    <div class="pb-3 select-text">
      <div
        aria-hidden="true"
        class={[
          "sticky top-0 z-10 flex h-8 items-center gap-2 border-b",
          "border-base-content/10 bg-base-100 pr-3 pl-4 text-xs",
          "text-base-content/70",
        ]}
      >
        <span class="flex-1">{m.plan_step()}</span>
        <span class="w-24 text-right">{m.plan_rows_head()}</span>
        <span class="w-24 text-right">
          {timed ? m.plan_time() : m.plan_cost()}
        </span>
      </div>

      <PlanTree node={query.plan.tree} {slowest} />
    </div>
  {:else}
    <pre
      class="px-4 py-3 text-sm leading-6 whitespace-pre-wrap select-text">{query
        .plan?.text ?? ""}</pre>
  {/if}
</div>
