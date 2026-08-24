<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon, ResultChart } from "@gpql/ui"

  import { workspace } from "$lib/session/workspace.svelte"

  import ResultGrid from "./ResultGrid.svelte"
  import TableList from "./TableList.svelte"

  let view = $state<"table" | "chart">("table")
</script>

<div class="flex h-full gap-2 p-2">
  <TableList />

  <section class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-baseline gap-2 px-4 pt-2 pb-1">
      <h2 class="text-sm font-medium">{workspace.selected ?? m.no_table()}</h2>

      <span class="text-xs text-base-content/45">
        {m.columns_count({ count: workspace.rows?.columns.length ?? 0 })}
      </span>

      <span class="flex-1"></span>

      <div class="flex gap-1 self-center rounded-selector bg-base-200 p-0.5">
        {#each [{ id: "table", icon: "lucide:table-2" }, { id: "chart", icon: "lucide:bar-chart-3" }] as option (option.id)}
          <button
            type="button"
            aria-label={option.id}
            aria-pressed={view === option.id}
            onclick={() => (view = option.id as "table" | "chart")}
            class="rounded-selector px-2 py-1 transition-colors {view ===
            option.id
              ? 'bg-base-100 text-base-content hairline'
              : 'text-base-content/45'}"
          >
            <Icon icon={option.icon} class="size-3.5" />
          </button>
        {/each}
      </div>
    </header>

    {#if view === "chart" && workspace.rows}
      <ResultChart
        columns={workspace.rows.columns}
        rows={workspace.rows.rows}
      />
    {:else}
      <ResultGrid result={workspace.rows} empty={m.pick_table()} />
    {/if}
  </section>
</div>
