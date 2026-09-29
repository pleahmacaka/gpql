<script lang="ts">
  import { type Snippet, untrack } from "svelte"
  import { fade } from "svelte/transition"

  import {
    DataGrid,
    EmptyState,
    Icon,
    Marker,
    type MenuItem,
    tooltip,
    veil,
  } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import type { Browse } from "$lib/session/browse.svelte"
  import { hint } from "$lib/session/errors"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { FilterOp, QueryResult, TableFilter } from "$lib/types"

  type Props = {
    result: QueryResult | null
    empty: string
    types?: Record<string, string>
    editable?: boolean
    spot?: { row: number; column: number } | null
    needle?: string
    onblocked?: (retry: () => void) => void
    browse?: Browse | null
    actions?: (row: number) => MenuItem[]
    status?: Snippet
  }

  let {
    result,
    empty,
    types = {},
    editable = false,
    spot = null,
    needle = "",
    onblocked,
    browse = null,
    actions,
    status,
  }: Props = $props()

  type GridSort = { column: string; dir: "asc" | "desc" }

  const OPS: FilterOp[] = [
    "contains",
    "eq",
    "ne",
    "gt",
    "gte",
    "lt",
    "lte",
    "starts",
    "ends",
    "isnull",
    "notnull",
  ]

  // only a server-sliced table can be ordered and filtered as a whole; a
  // query result is already complete, so the grid ranks that one itself
  let remote = $derived(browse?.serverSide ? browse : null)

  let owner = $derived(workspace.active?.id ?? "")

  let source = $derived(
    browse ? `${owner}/table/${browse.table ?? ""}` : `${owner}/query`,
  )

  let heldFor = $state<string | null>(null)

  $effect.pre(() => {
    void result

    untrack(() => {
      heldFor = browse?.table ?? null
    })
  })

  let fresh = $derived(
    !!browse && browse.busy && !!browse.table && browse.table !== heldFor,
  )

  let columns = $derived(result?.columns ?? [])

  let failure = $derived(browse?.error ?? "")
  let advice = $derived(failure ? hint(failure) : "")

  let sort = $derived.by((): GridSort | null | undefined => {
    if (!remote) {
      return undefined
    }

    const held = remote.sort

    return held
      ? { column: held.column, dir: held.descending ? "desc" : "asc" }
      : null
  })

  function toFilters(filters: Record<string, { op: string; value: string }>) {
    const out: Record<string, TableFilter> = {}

    for (const [column, filter] of Object.entries(filters)) {
      const op = OPS.find(entry => entry === filter.op)

      if (op) {
        out[column] = {
          op,
          value: filter.value,
          needsValue: op !== "isnull" && op !== "notnull",
        }
      }
    }

    return out
  }

  let labels = $derived({
    copyCell: m.menu_copy(),
    copyRow: m.menu_copy_row(),
    copyColumn: m.menu_copy_name(),
    copyAll: m.menu_copy_all(),
    inspect: m.menu_inspect(),
    jumpTo: m.menu_jump_to(),
    filterBy: m.menu_filter_by(),
    clearFilters: m.menu_clear_filters(),
    dropFilter: m.filter_drop(),
    contains: m.filter_contains(),
    starts: m.filter_starts(),
    ends: m.filter_ends(),
    isnull: m.filter_isnull(),
    notnull: m.filter_notnull(),
    apply: m.apply(),
    discard: m.discard(),
    edited: m.edited(),
    noKey: m.no_key(),
    value: m.filter_value(),
    loading: m.loading_rows(),
    pretty: m.pretty_print(),
    cancel: m.cancel(),
    close: m.close(),
    sort: m.sort(),
    resize: m.resize(),
    ascending: m.sort_ascending(),
    descending: m.sort_descending(),
    unsorted: m.sort_none(),
    filter: m.filter_label(),
    edit: m.cell_edit(),
    filters: m.filters_active(),
  })
</script>

{#snippet footer()}
  {#if failure}
    <span
      use:tooltip={advice ? `${failure}\n${advice}` : failure}
      class="flex min-w-0 items-center gap-2 text-error"
    >
      <Icon icon="lucide:circle-alert" class="size-4 shrink-0" />
      <span class="truncate select-text">{failure}</span>
    </span>
  {/if}

  {#if status}
    {@render status()}
  {/if}
{/snippet}

{#if fresh || columns.length > 0}
  <DataGrid
    {columns}
    rows={result?.rows ?? []}
    {source}
    {types}
    loading={fresh}
    empty={browse ? empty : ""}
    rowHeight={workspace.rowHeight}
    {editable}
    locked={!workspace.writable}
    keyColumns={editable ? workspace.keyColumns : []}
    busy={browse ? browse.busy : workspace.query.busy}
    minimap={workspace.minimap}
    {spot}
    {needle}
    {sort}
    filters={remote ? remote.filters : undefined}
    bind:pretty={() => workspace.pretty, on => workspace.setPretty(on)}
    status={footer}
    editCount={count => m.cells_edited({ count })}
    onapply={edits => workspace.applyEdits(edits)}
    {onblocked}
    onsort={remote
      ? next =>
          remote.setSort(
            next
              ? { column: next.column, descending: next.dir === "desc" }
              : null,
          )
      : undefined}
    onfilter={remote
      ? filters => remote.setFilters(toFilters(filters))
      : undefined}
    onmore={browse ? () => browse.more() : undefined}
    onjump={browse
      ? (column, value) => workspace.jumpTo(column, value)
      : undefined}
    {actions}
    references={browse ? workspace.references : {}}
    more={browse ? !browse.end : false}
    paging={browse?.paging ?? false}
    {labels}
  />
{:else}
  <div class="flex min-h-0 flex-1 flex-col">
    {#if failure}
      <div
        in:fade|local={veil()}
        class="grid min-h-0 flex-1 place-items-center p-8"
      >
        <div
          role="alert"
          class={[
            "hud hud-lit hud-error flex w-lg max-w-full flex-col gap-3",
            "p-6",
          ]}
        >
          <Marker label={m.rows_failed()} tone="error" as="h3" />

          <p class="text-sm wrap-anywhere select-text">{failure}</p>

          {#if advice}
            <p class="text-xs text-base-content/70">{advice}</p>
          {/if}

          {#if browse}
            <div>
              <button
                type="button"
                onclick={() => browse.reload()}
                class="btn btn-soft btn-sm"
              >
                <Icon icon="lucide:rotate-cw" class="size-4" />
                {m.retry()}
              </button>
            </div>
          {/if}
        </div>
      </div>
    {:else if result?.affected != null}
      <p
        in:fade|local={veil()}
        role="status"
        class={[
          "flex items-center gap-2 px-4 py-3 text-sm",
          "text-base-content/70",
        ]}
      >
        <Icon icon="lucide:check" class="size-4 shrink-0 text-success" />
        {m.rows_touched({ count: result.affected })}
      </p>
    {:else if empty !== ""}
      <div
        in:fade|local={veil()}
        class="grid min-h-0 flex-1 place-items-center"
      >
        <EmptyState art="sheet" title={empty} />
      </div>
    {/if}
  </div>

  {#if status}
    <footer
      class={[
        "flex h-10 shrink-0 items-center gap-3 border-t border-base-content/10",
        "px-3 text-xs text-base-content/70 tabular-nums",
      ]}
    >
      <span class="size-4 shrink-0"></span>

      <div class="flex min-w-0 flex-1 items-center gap-3 overflow-hidden">
        {@render status()}
      </div>
    </footer>
  {/if}
{/if}
