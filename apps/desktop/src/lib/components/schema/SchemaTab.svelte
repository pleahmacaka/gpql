<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { SvelteFlowProvider } from "@xyflow/svelte"

  import TableList from "$lib/components/data/TableList.svelte"
  import { workspace } from "$lib/session/workspace.svelte"

  import FindBar from "$lib/components/shell/FindBar.svelte"
  import TabLayout from "$lib/components/shell/TabLayout.svelte"

  import { Icon, board, relationCount } from "@gpql/ui"
  import DiffPanel from "./DiffPanel.svelte"
  import SchemaBoard from "./SchemaBoard.svelte"
  import SharePanel from "./SharePanel.svelte"

  let relations = $derived(relationCount(workspace.schema))

  let diffing = $state(false)
  let sharing = $state(false)
  let describing = $state<AbortController | null>(null)

  async function annotate() {
    const provider = workspace.model
    const here = workspace.active

    if (describing) {
      describing.abort()

      return
    }

    if (!provider || !here) {
      return
    }

    const stopper = new AbortController()

    describing = stopper

    try {
      await here.describe(provider, stopper.signal)
    } catch (failure) {
      if (!stopper.signal.aborted) {
        workspace.notice = String(failure)
      }
    } finally {
      if (describing === stopper) {
        describing = null
      }
    }
  }
  let term = $state("")
  let hit = $state(0)

  // a table matches on its own name or on any column it carries
  let hits = $derived.by(() => {
    const needle = term.trim().toLowerCase()

    if (needle === "") {
      return []
    }

    return workspace.schema
      .filter(
        table =>
          table.name.toLowerCase().includes(needle) ||
          table.columns.some(column =>
            column.name.toLowerCase().includes(needle),
          ),
      )
      .map(table => table.name)
  })

  $effect(() => {
    term
    hit = 0
  })

  $effect(() => {
    board.needle = workspace.finding ? term.trim().toLowerCase() : ""

    return () => {
      board.needle = ""
    }
  })

  $effect(() => {
    const names = hits

    if (!workspace.finding || names.length === 0) {
      return
    }

    board.selected = names[Math.min(hit, names.length - 1)]
  })

  function step(by: number) {
    if (hits.length > 0) {
      hit = (hit + by + hits.length) % hits.length
    }
  }
</script>

<TabLayout>
  {#snippet aside()}
    <TableList />
  {/snippet}

  <section class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-center gap-2 px-4 pt-2 pb-1">
      <h2 class="text-sm font-medium">Schema</h2>

      <span class="text-xs text-base-content/45">
        {m.tables_count({ count: workspace.schema.length })}
      </span>

      <span class="text-xs text-base-content/45">{m.relations_count({ count: relations })}</span>

      <span class="flex-1"></span>

      {#if workspace.finding}
        <FindBar
          placeholder={m.find_tables()}
          bind:term
          index={hit}
          total={hits.length}
          onnext={() => step(1)}
          onprev={() => step(-1)}
          onclose={() => (workspace.finding = false)}
        />
      {/if}

      <button
        type="button"
        aria-pressed={sharing}
        onclick={() => (sharing = !sharing)}
        class="flex items-center gap-2 rounded-field px-2 py-1 text-xs
          {sharing ? 'bg-primary/10 text-primary' : 'bg-base-200 hover:bg-base-300'}"
      >
        <Icon icon="lucide:share-2" class="size-4" />
        {m.share_erd()}
      </button>

      {#if workspace.ai && workspace.model && workspace.active}
        <button
          type="button"
          onclick={annotate}
          class="flex items-center gap-2 rounded-field bg-base-200 px-2 py-1
            text-xs hover:bg-base-300"
        >
          <Icon
            icon={describing ? "lucide:loader-circle" : "lucide:text-quote"}
            class="size-4 {describing ? 'animate-spin' : ''}"
          />
          {describing ? m.cancel() : m.schema_describe()}
        </button>
      {/if}

      {#if workspace.connections.length > 1}
        <button
          type="button"
          aria-pressed={diffing}
          onclick={() => (diffing = !diffing)}
          class="flex items-center gap-2 rounded-field px-2 py-1 text-xs
            {diffing
            ? 'bg-primary/10 text-primary'
            : 'bg-base-200 hover:bg-base-300'}"
        >
          <Icon icon="lucide:git-compare" class="size-4" />
          {m.tab_diff()}
        </button>
      {/if}
    </header>

    <div class="min-h-0 flex-1 overflow-hidden rounded-box">
      {#if diffing}
        <DiffPanel />
      {:else}
        <SvelteFlowProvider>
          <SchemaBoard />
        </SvelteFlowProvider>
      {/if}
    </div>
  </section>
</TabLayout>

{#if sharing}
  <SharePanel onclose={() => (sharing = false)} />
{/if}
