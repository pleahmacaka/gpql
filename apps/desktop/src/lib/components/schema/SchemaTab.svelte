<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { SvelteFlowProvider } from "@xyflow/svelte"

  import TableList from "$lib/components/data/TableList.svelte"
  import { workspace } from "$lib/session/workspace.svelte"

  import FindBar from "$lib/components/shell/FindBar.svelte"
  import TabLayout from "$lib/components/shell/TabLayout.svelte"
  import { site } from "$lib/sync/client"

  import { board, relationCount } from "@gpql/ui"
  import SchemaBoard from "./SchemaBoard.svelte"

  let relations = $derived(relationCount(workspace.schema))

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

      <span class="text-xs text-base-content/40">{m.arrows_walk()}</span>

      {#if workspace.signedIn}
        <button
          type="button"
          onclick={() => workspace.publish(site)}
          class="rounded-field px-2 py-0.5 text-xs text-primary hover:bg-base-200"
        >
          {m.share_erd()}
        </button>
      {:else}
        <span class="text-xs text-base-content/35">{m.share_needs_login()}</span>
      {/if}

      {#if workspace.shared}
        <span class="max-w-56 truncate font-mono text-xs text-base-content/40">
          {workspace.shared}
        </span>
      {/if}

    </header>

    <div class="min-h-0 flex-1 overflow-hidden rounded-box">
      <SvelteFlowProvider>
        <SchemaBoard />
      </SvelteFlowProvider>
    </div>
  </section>
</TabLayout>
