<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import ResultGrid from "$lib/components/data/ResultGrid.svelte"
  import { Dropdown, Icon, Keycap } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  import FindBar from "$lib/components/shell/FindBar.svelte"
  import TabLayout from "$lib/components/shell/TabLayout.svelte"

  import AskDialog from "./AskDialog.svelte"
  import SavedQueries from "./SavedQueries.svelte"
  import SqlEditor from "./SqlEditor.svelte"

  let asking = $state(false)
  let term = $state("")
  let hit = $state(0)
  let editor = $state<SqlEditor | null>(null)

  let hits = $derived.by(() => {
    const needle = term.trim().toLowerCase()

    if (needle === "") {
      return []
    }

    const hay = workspace.sql.toLowerCase()
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
    workspace.selection = { start: 0, end: 0 }
  }

  let picked = $derived(workspace.selection.end > workspace.selection.start)

  let limits = $derived(
    workspace.limits.map(rows => ({
      value: String(rows),
      label: m.rows_count({ count: rows }),
    })),
  )

  let status = $derived(
    workspace.queryError
      ? workspace.queryError
      : workspace.busy
        ? m.running()
        : workspace.queryRan
          ? m.rows_count({ count: workspace.queryResult?.rows.length ?? 0 })
          : m.nothing_run(),
  )

  // an open saved query keeps itself up to date a moment after typing stops
  $effect(() => {
    const text = workspace.sql
    const id = workspace.openQuery
    const stored = workspace.saved.find(entry => entry.id === id)?.sql

    // just opening a query is not an edit
    if (!id || text.trim() === "" || text === stored) {
      return
    }

    workspace.autosaved = false

    const timer = setTimeout(async () => {
      await workspace.keep()
      workspace.autosaved = true
    }, 1200)

    return () => clearTimeout(timer)
  })
</script>

<TabLayout>
  {#snippet aside()}
    <SavedQueries />
  {/snippet}

  <section class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-center gap-3 px-4 pt-2 pb-1">
      <h2 class="text-sm font-medium">Query</h2>

      {#if workspace.autosaved && workspace.openQuery}
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

      <span class="flex items-center gap-1.5">
        <Icon icon="lucide:rows-4" class="size-3.5 text-base-content/35" />

        <span class="text-xs text-base-content/40">{m.row_limit()}</span>

        <Dropdown
          options={limits}
          value={String(workspace.rowLimit)}
          onpick={rows => workspace.setRowLimit(Number(rows))}
        />
      </span>

      {#if workspace.ai && workspace.providers.length > 0}
        <button
          type="button"
          onclick={() => (asking = true)}
          class="flex items-center gap-1.5 rounded-field bg-base-200 px-2 py-1
            text-xs transition-colors hover:bg-base-300"
        >
          <Icon icon="lucide:sparkles" class="size-3.5 text-accent" />
          {m.ai_write()}
        </button>
      {/if}

      <button
        type="button"
        onclick={() => workspace.run()}
        disabled={workspace.busy || workspace.sql.trim() === ""}
        class="flex items-center gap-1.5 rounded-field bg-primary px-2 py-1
          text-xs text-primary-content transition-colors hover:bg-primary/90
          disabled:bg-base-200 disabled:text-base-content/30"
      >
        <Icon icon="lucide:play" class="size-3.5" />
        {picked ? m.run_picked() : m.run_all()}
        <Keycap keys={["ctrl", "enter"]} class="opacity-70" />
      </button>
    </header>

    <div class="mx-3 rounded-field bg-base-200 px-3 pt-2 pb-1.5">
      <SqlEditor
        bind:this={editor}
        bind:value={workspace.sql}
        bind:selection={workspace.selection}
        onrun={() => workspace.run()}
        onclear={() => workspace.clearQuery()}
      />

      <p
        class="flex items-center gap-1.5 pt-1 text-xs {workspace.queryError
          ? 'text-error'
          : 'text-base-content/40'}"
      >
        <Icon icon="lucide:play" class="size-3" />
        {status}
      </p>
    </div>

    <ResultGrid result={workspace.queryResult} empty="" />
  </section>
</TabLayout>

{#if asking}
  <AskDialog onclose={() => (asking = false)} />
{/if}
