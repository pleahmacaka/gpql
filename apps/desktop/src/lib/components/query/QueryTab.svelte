<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import ResultGrid from "$lib/components/data/ResultGrid.svelte"
  import { Dropdown, Icon, Keycap, Lazy, menu } from "@gpql/ui"
  import { FORMATS, exportResult } from "$lib/session/exporting"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { ExportFormat } from "$lib/types"

  import FindBar from "$lib/components/shell/FindBar.svelte"
  import TabLayout from "$lib/components/shell/TabLayout.svelte"

  import AskDialog from "./AskDialog.svelte"
  import QueryBuilder from "./QueryBuilder.svelte"
  import SavedQueries from "./SavedQueries.svelte"
  import PlanPanel from "./PlanPanel.svelte"
  import SqlEditor from "./SqlEditor.svelte"

  let asking = $state(false)
  let building = $state(false)
  let charting = $state(false)
  let term = $state("")
  let hit = $state(0)
  let editor = $state<SqlEditor | null>(null)

  let hits = $derived.by(() => {
    const needle = term.trim().toLowerCase()

    if (needle === "") {
      return []
    }

    const hay = workspace.query.sql.toLowerCase()
    const out: number[] = []
    let at = hay.indexOf(needle)

    while (at !== -1) {
      out.push(at)
      at = hay.indexOf(needle, at + needle.length)
    }

    return out
  })

  // typing only updates the count; the editor is touched on explicit steps,
  // otherwise reveal() would steal the focus on every keystroke
  $effect(() => {
    term
    hit = 0
  })

  function show() {
    const at = hits[Math.min(hit, hits.length - 1)]

    if (at !== undefined) {
      editor?.reveal(at, at + term.trim().length)
    }
  }

  function step(by: number) {
    if (hits.length > 0) {
      hit = (hit + by + hits.length) % hits.length
      show()
    }
  }

  function doneFinding() {
    workspace.finding = false
    workspace.query.selection = { start: 0, end: 0 }
  }

  let picked = $derived(workspace.query.selection.end > workspace.query.selection.start)

  let limits = $derived(
    workspace.limits.map(rows => ({
      value: String(rows),
      label: m.rows_count({ count: rows }),
    })),
  )

  let status = $derived(
    workspace.query.error
      ? workspace.query.error
      : workspace.busy
        ? m.running()
        : workspace.query.ran
          ? m.rows_count({ count: workspace.query.result?.rows.length ?? 0 })
          : m.nothing_run(),
  )

  // an open saved query keeps itself up to date a moment after typing stops
  $effect(() => {
    const text = workspace.query.sql
    const id = workspace.query.open
    const stored = workspace.query.saved.find(entry => entry.id === id)?.sql

    // just opening a query is not an edit
    if (!id || text.trim() === "" || text === stored) {
      return
    }

    workspace.query.autosaved = false

    const timer = setTimeout(async () => {
      await workspace.query.keep()
      workspace.query.autosaved = true
    }, 1200)

    return () => clearTimeout(timer)
  })

  // influx opens on the builder, the way its own data explorer does
  $effect(() => {
    if (workspace.dialect === "flux") {
      building = true
      void workspace.loadSchema()
    }
  })

  async function build() {
    building = !building

    if (building) {
      await workspace.loadSchema()
    }
  }

  async function shipOut(format: ExportFormat) {
    const session = workspace.session
    const result = workspace.query.result

    if (!session || !result) {
      return
    }

    try {
      workspace.notice =
        (await exportResult(session.id, result, format)) ?? ""
    } catch (failure) {
      workspace.notice = m.export_failed({ reason: String(failure) })
    }
  }
</script>

<TabLayout>
  {#snippet aside()}
    <SavedQueries />
  {/snippet}

  <section class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-center gap-3 px-4 pt-2 pb-1">
      <h2 class="text-sm font-medium">Query</h2>

      {#if workspace.query.autosaved && workspace.query.open}
        <span class="text-xs text-base-content/35">{m.autosaved()}</span>
      {/if}

      <span class="flex-1"></span>

      {#if workspace.finding}
        <FindBar
          placeholder={m.find_sql()}
          bind:term
          index={hit}
          total={hits.length}
          onnext={() => step(1)}
          onprev={() => step(-1)}
          onclose={doneFinding}
        />
      {/if}

      <span class="flex items-center gap-2">
        <Icon icon="lucide:rows-4" class="size-4 text-base-content/35" />

        <span class="text-xs text-base-content/40">{m.row_limit()}</span>

        <Dropdown
          options={limits}
          value={String(workspace.rowLimit)}
          onpick={rows => workspace.setRowLimit(Number(rows))}
        />
      </span>

      {#if workspace.session?.sliceable}
        <span class="flex items-center gap-1 rounded-field bg-base-200 p-1">
          <button
            type="button"
            onclick={() => building || build()}
            aria-pressed={building}
            class="flex items-center gap-2 rounded-field px-2 py-1 text-xs
              transition-colors {building
              ? 'bg-primary/10 text-primary'
              : 'hover:bg-base-300'}"
          >
            <Icon icon="lucide:blocks" class="size-4" />
            {m.mode_builder()}
          </button>

          <button
            type="button"
            onclick={() => building && build()}
            aria-pressed={!building}
            class="flex items-center gap-2 rounded-field px-2 py-1 text-xs
              transition-colors {building
              ? 'hover:bg-base-300'
              : 'bg-primary/10 text-primary'}"
          >
            <Icon icon="lucide:code" class="size-4" />
            {m.mode_script()}
          </button>
        </span>
      {/if}

      {#if workspace.query.result && workspace.query.result.rows.length > 0}
        <button
          type="button"
          onclick={() => (charting = !charting)}
          aria-pressed={charting}
          class="flex items-center gap-2 rounded-field px-2 py-1 text-xs
            transition-colors {charting
            ? 'bg-primary/10 text-primary'
            : 'bg-base-200 hover:bg-base-300'}"
        >
          <Icon
            icon={charting ? "lucide:table" : "lucide:chart-line"}
            class="size-4"
          />
          {charting ? m.view_table() : m.view_chart()}
        </button>
      {/if}

      {#if workspace.ai && workspace.providers.length > 0}
        <button
          type="button"
          onclick={() => (asking = true)}
          class="flex items-center gap-2 rounded-field bg-base-200 px-2 py-1
            text-xs transition-colors hover:bg-base-300"
        >
          <Icon icon="lucide:sparkles" class="size-4 text-accent" />
          {m.ai_write()}
        </button>
      {/if}

      <button
        type="button"
        disabled={workspace.query.busy || workspace.query.chosen === ""}
        onclick={event =>
          menu.show(event, [
            {
              label: m.menu_explain(),
              icon: "lucide:git-fork",
              run: () => workspace.query.explain(false),
            },
            {
              label: m.menu_explain_analyze(),
              icon: "lucide:timer",
              run: () => workspace.query.explain(true),
            },
          ])}
        class="flex items-center gap-2 rounded-field bg-base-200 px-2 py-1
          text-xs hover:bg-base-300 disabled:opacity-40"
      >
        <Icon icon="lucide:git-fork" class="size-4" />
        {m.menu_explain()}
      </button>

      {#if workspace.query.result && workspace.query.result.columns.length > 0}
        <button
          type="button"
          aria-label={m.menu_export()}
          onclick={event =>
            menu.show(
              event,
              FORMATS.map(format => ({
                label: m.menu_export_as({ format: format.toUpperCase() }),
                icon: "lucide:download",
                run: () => shipOut(format),
              })),
            )}
          class="flex items-center gap-2 rounded-field bg-base-200 px-2 py-1
            text-xs hover:bg-base-300"
        >
          <Icon icon="lucide:download" class="size-4" />
          {m.menu_export()}
        </button>
      {/if}

      <button
        type="button"
        onclick={() => {
          workspace.query.spot = false
          workspace.query.run()
        }}
        disabled={workspace.busy || workspace.query.sql.trim() === ""}
        class="flex items-center gap-2 rounded-field bg-primary px-2 py-1
          text-xs text-primary-content transition-colors hover:bg-primary/90
          disabled:bg-base-200 disabled:text-base-content/30
          {workspace.query.spot
          ? 'animate-pulse ring-2 ring-accent ring-offset-2 ring-offset-base-100'
          : ''}"
      >
        <Icon icon="lucide:play" class="size-4" />
        {picked ? m.run_picked() : m.run_all()}
        <Keycap keys={["ctrl", "enter"]} class="opacity-70" />
      </button>
    </header>

    {#if building}
      <QueryBuilder />
    {/if}

    <div
      class="mx-3 rounded-field bg-base-200 px-3 pt-2 pb-2 {building
        ? 'hidden'
        : ''}"
    >
      <SqlEditor
        bind:this={editor}
        bind:value={workspace.query.sql}
        bind:selection={workspace.query.selection}
        onrun={() => workspace.query.run()}
        onclear={() => workspace.query.clear()}
      />

      <p
        class="flex items-center gap-2 pt-1 text-xs {workspace.query.error
          ? 'text-error'
          : 'text-base-content/40'}"
      >
        <Icon icon="lucide:play" class="size-3" />
        {status}
      </p>
    </div>

    {#if workspace.query.plan}
      <PlanPanel />
    {:else if charting && workspace.query.result}
      <Lazy
        load={() => import("@gpql/ui/data/ResultChart.svelte")}
        props={{
          columns: workspace.query.result.columns,
          rows: workspace.query.result.rows,
        }}
      />
    {:else}
      <ResultGrid result={workspace.query.result} empty="" />
    {/if}
  </section>
</TabLayout>

{#if asking}
  <AskDialog onclose={() => (asking = false)} />
{/if}
