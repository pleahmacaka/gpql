<script lang="ts">
  import {
    arrive,
    board,
    depart,
    EmptyState,
    Icon,
    Panel,
    relationCount,
    Segmented,
    tooltip,
    veil,
  } from "@gpql/ui"
  import { SvelteFlowProvider } from "@xyflow/svelte"
  import { untrack } from "svelte"
  import { fade } from "svelte/transition"

  import TableList from "$lib/components/data/TableList.svelte"
  import FindBar from "$lib/components/shell/FindBar.svelte"
  import TabLayout from "$lib/components/shell/TabLayout.svelte"
  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  import DiffPanel from "./DiffPanel.svelte"
  import SchemaBoard from "./SchemaBoard.svelte"
  import SharePanel from "./SharePanel.svelte"

  let view = $state("board")
  let sharing = $state(false)
  let opener = $state<HTMLButtonElement | null>(null)
  let describing = $state<AbortController | null>(null)

  let here = $derived(workspace.active)
  let phase = $derived(here?.schemaState ?? "idle")
  let filled = $derived(workspace.schema.length > 0)
  let comparing = $derived(workspace.connections.length > 1)
  let diffing = $derived(view === "diff" && comparing)
  let shown = $derived(workspace.tab === "schema")
  let relations = $derived(relationCount(workspace.schema))

  $effect(() => {
    const target = here

    if (shown && target?.schemaState === "idle") {
      untrack(() => void target.loadSchema())
    }
  })

  async function annotate() {
    const provider = workspace.model
    const target = here

    if (describing) {
      describing.abort()

      return
    }

    if (!provider || !target) {
      return
    }

    const stopper = new AbortController()

    describing = stopper

    try {
      await target.describe(provider, stopper.signal)
    } catch (failure) {
      if (!stopper.signal.aborted) {
        workspace.error = String(failure)
      }
    } finally {
      if (describing === stopper) {
        describing = null
      }
    }
  }

  let term = $state("")
  let hit = $state(0)

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
    void term
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

{#snippet swap(first: string, second: string, flipped: boolean)}
  <span class="grid">
    <span class={["col-start-1 row-start-1", flipped && "invisible"]}>
      {first}
    </span>

    <span class={["col-start-1 row-start-1", !flipped && "invisible"]}>
      {second}
    </span>
  </span>
{/snippet}

<TabLayout>
  {#snippet aside()}
    <TableList />
  {/snippet}

  <Panel glass={false} label={m.tab_schema()} class="min-w-0 flex-1">
    <div class="@container shrink-0 border-b border-base-content/10">
      <header class="flex h-12 items-center gap-2 px-4">
        <h2 class="shrink-0 text-sm font-semibold">{m.tab_schema()}</h2>

        {#if !workspace.finding}
          <p
            class={[
              "hidden min-w-0 items-center gap-3 text-xs whitespace-nowrap",
              "text-base-content/70 tabular-nums @2xl:flex",
            ]}
          >
            <span>{m.tables_count({ count: workspace.schema.length })}</span>
            <span>{m.relations_count({ count: relations })}</span>

            {#if phase === "loading" && filled}
              <span class="flex items-center gap-1" in:fade={veil()}>
                <Icon icon="lucide:loader-circle" class="size-3 animate-spin" />
                {m.schema_loading()}
              </span>
            {/if}
          </p>
        {/if}

        <span class="flex-1"></span>

        {#if workspace.finding && !diffing}
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

        {#if workspace.ai && workspace.model && here}
          <button
            type="button"
            onclick={annotate}
            aria-busy={describing !== null}
            aria-label={describing ? m.cancel() : m.schema_describe()}
            use:tooltip={m.schema_describe()}
            class="btn btn-ghost btn-sm shrink-0 font-medium"
          >
            <Icon
              icon={describing ? "lucide:loader-circle" : "lucide:text-quote"}
              class={["size-4", describing && "animate-spin"]}
            />

            <span class="hidden @3xl:inline">
              {@render swap(m.schema_describe(), m.cancel(), !!describing)}
            </span>
          </button>
        {/if}

        <button
          bind:this={opener}
          type="button"
          aria-haspopup="dialog"
          aria-expanded={sharing}
          aria-label={m.share_erd()}
          use:tooltip={m.share_erd()}
          onclick={() => (sharing = !sharing)}
          class={[
            "btn btn-sm shrink-0 font-medium",
            sharing ? "btn-soft btn-primary" : "btn-ghost",
          ]}
        >
          <Icon icon="lucide:share-2" class="size-4" />
          <span class="hidden @3xl:inline">{m.share_erd()}</span>
        </button>

        {#if comparing}
          <Segmented
            small
            label={m.schema_view()}
            bind:value={view}
            options={[
              {
                value: "board",
                label: m.schema_diagram(),
                icon: "lucide:workflow",
              },
              {
                value: "diff",
                label: m.tab_diff(),
                icon: "lucide:git-compare",
              },
            ]}
          />
        {/if}
      </header>
    </div>

    <div class="relative min-h-0 flex-1 overflow-hidden">
      {#if phase === "failed" && here}
        <div
          in:fade={veil()}
          class="absolute inset-0 grid place-items-center overflow-y-auto"
        >
          <EmptyState
            art="link"
            title={m.schema_failed()}
            hint={here.schemaError}
            class="select-text"
          >
            <button
              type="button"
              onclick={() => here?.loadSchema(true)}
              class="btn btn-soft btn-sm font-medium"
            >
              <Icon icon="lucide:rotate-cw" class="size-4" />
              {m.diff_retry()}
            </button>
          </EmptyState>
        </div>
      {:else if filled}
        <div in:fade={veil()} class="absolute inset-0">
          <SvelteFlowProvider>
            <SchemaBoard keyboard={shown && !diffing && !sharing} />
          </SvelteFlowProvider>
        </div>
      {:else if phase === "ready"}
        <div in:fade={veil()} class="absolute inset-0 grid place-items-center">
          <EmptyState
            art="graph"
            title={m.schema_empty()}
            hint={m.schema_empty_hint()}
          />
        </div>
      {:else}
        <div
          role="status"
          in:fade={veil()}
          class="absolute inset-0 grid place-items-center"
        >
          <EmptyState art="graph" title={m.schema_loading()} />
        </div>
      {/if}

      {#if diffing}
        <div
          in:arrive={{ from: "right", distance: 1 }}
          out:depart={{ to: "right", distance: 1 }}
          class="absolute inset-0 flex flex-col bg-base-100"
        >
          <DiffPanel />
        </div>
      {/if}
    </div>
  </Panel>
</TabLayout>

{#if sharing && opener}
  <SharePanel anchor={opener} onclose={() => (sharing = false)} />
{/if}
