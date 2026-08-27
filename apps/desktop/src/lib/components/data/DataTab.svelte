<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { fade, scale } from "svelte/transition"

  import { Icon, Lazy, pop, veil } from "@gpql/ui"

  import { workspace } from "$lib/session/workspace.svelte"

  import FindBar from "$lib/components/shell/FindBar.svelte"
  import TabLayout from "$lib/components/shell/TabLayout.svelte"

  import ResultGrid from "./ResultGrid.svelte"
  import TableList from "./TableList.svelte"

  let view = $state<"table" | "chart">("table")
  let asking = $state(false)
  let term = $state("")
  let hit = $state(0)

  let hits = $derived.by(() => {
    const needle = term.trim().toLowerCase()
    const result = workspace.rows

    if (needle === "" || !result) {
      return []
    }

    const out: { row: number; column: number }[] = []

    result.rows.forEach((row, y) => {
      row.forEach((cell, x) => {
        if ((cell ?? "").toLowerCase().includes(needle)) {
          out.push({ row: y, column: x })
        }
      })
    })

    return out
  })

  let bump = $state(0)

  let spot = $derived.by(() => {
    bump

    if (!workspace.finding) {
      return null
    }

    const found = hits[Math.min(hit, hits.length - 1)]

    return found ? { ...found } : null
  })

  $effect(() => {
    term
    hit = 0
  })

  function step(by: number) {
    if (hits.length > 0) {
      hit = (hit + by + hits.length) % hits.length
      bump++
    }
  }

  function toggleWrites() {
    if (workspace.readOnly) {
      asking = true

      return
    }

    workspace.toggle("readOnly")
  }

  async function allowWrites() {
    asking = false
    await workspace.toggle("readOnly")
  }
</script>

<TabLayout>
  {#snippet aside()}
    <TableList />
  {/snippet}

  <section class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-center gap-2 px-4 pt-2 pb-1">
      <h2 class="text-sm font-medium">{workspace.selected ?? m.no_table()}</h2>

      <span class="text-xs text-base-content/45">
        {m.columns_count({ count: workspace.rows?.columns.length ?? 0 })}
      </span>

      <span class="flex-1"></span>

      {#if workspace.finding}
        <FindBar
          placeholder={m.find_rows()}
          bind:term
          index={hit}
          total={hits.length}
          onnext={() => step(1)}
          onprev={() => step(-1)}
          onclose={() => (workspace.finding = false)}
        />
      {/if}

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

      <button
        type="button"
        onclick={toggleWrites}
        aria-pressed={!workspace.readOnly}
        title={workspace.readOnly ? m.read_only() : m.writes_on()}
        class="flex items-center gap-1.5 self-center rounded-selector px-2 py-1
          text-xs transition-colors {workspace.readOnly
          ? 'bg-base-200 text-base-content/55 hover:bg-base-300'
          : 'bg-warning/15 text-warning'}"
      >
        <Icon
          icon={workspace.readOnly ? "lucide:lock" : "lucide:pencil"}
          class="size-3.5"
        />
        {workspace.readOnly ? m.read_only() : m.writes_on()}
      </button>
    </header>

    {#if view === "chart" && workspace.rows}
      <Lazy
        load={() => import("@gpql/ui/data/ResultChart.svelte")}
        props={{
          columns: workspace.rows.columns,
          rows: workspace.rows.rows,
        }}
      />
    {:else}
      <ResultGrid
        result={workspace.rows}
        empty={workspace.selected ? m.no_rows() : m.pick_table()}
        types={workspace.columnTypes}
        {spot}
        needle={workspace.finding ? term.trim().toLowerCase() : ""}
        editable
        onblocked={() => (asking = true)}
      />
    {/if}
  </section>
</TabLayout>

{#if asking}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    transition:fade={veil()}
    onclick={() => (asking = false)}
    class="fixed inset-0 z-50 scrim"
  ></div>

  <div
    transition:scale={pop()}
    role="dialog"
    aria-modal="true"
    class="fixed inset-x-0 top-1/3 z-50 mx-auto w-96 max-w-11/12 rounded-box
      floating p-5 lift"
  >
    <div class="flex items-start gap-3">
      <Icon icon="lucide:pencil" class="mt-0.5 size-4 shrink-0 text-warning" />

      <div>
        <h2 class="text-sm font-medium">{m.writes_ask()}</h2>

        <p class="pt-1 text-xs text-base-content/60">{m.writes_ask_hint()}</p>
      </div>
    </div>

    <div class="flex gap-2 pt-5">
      <button
        type="button"
        onclick={() => (asking = false)}
        class="flex-1 rounded-field bg-base-200 py-1.5 text-sm
          hover:bg-base-300"
      >
        {m.cancel()}
      </button>

      <button
        type="button"
        onclick={allowWrites}
        class="flex-1 rounded-field bg-warning py-1.5 text-sm
          text-warning-content"
      >
        {m.writes_allow()}
      </button>
    </div>
  </div>
{/if}
