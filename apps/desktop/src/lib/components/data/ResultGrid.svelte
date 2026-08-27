<script lang="ts">
  import { DataGrid } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Browse } from "$lib/session/browse.svelte"
  import type { QueryResult, TableFilter } from "$lib/types"

  type Props = {
    result: QueryResult | null
    empty: string
    types?: Record<string, string>
    editable?: boolean
    spot?: { row: number; column: number } | null
    needle?: string
    onblocked?: () => void
    browse?: Browse | null
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
  }: Props = $props()

  const NO_VALUE = ["isnull", "notnull"]

  // only a server-sliced table can be ordered and filtered as a whole; a
  // query result is already complete, so the grid ranks that one itself
  let remote = $derived(browse?.serverSide ? browse : null)
</script>

{#if !result || result.columns.length === 0}
  <p class="px-4 py-3 text-sm text-base-content/40">
    {result?.affected != null ? `${result.affected} rows touched` : empty}
  </p>
{:else}
  <DataGrid
    columns={result.columns}
    rows={result.rows}
    {types}
    rowHeight={workspace.rowHeight}
    editable={editable && workspace.writable}
    keyColumns={editable ? workspace.keyColumns : []}
    busy={browse ? browse.busy : workspace.query.busy}
    minimap={workspace.minimap}
    {spot}
    {needle}
    onapply={edits => workspace.applyEdits(edits)}
    {onblocked}
    onsort={remote
      ? sort =>
          remote.setSort(
            sort ? { column: sort.column, descending: sort.dir === "desc" } : null,
          )
      : undefined}
    onfilter={remote
      ? filters =>
          remote.setFilters(
            Object.fromEntries(
              Object.entries(filters).map(([column, filter]) => [
                column,
                { ...filter, needsValue: !NO_VALUE.includes(filter.op) },
              ]),
            ) as Record<string, TableFilter>,
          )
      : undefined}
    onmore={browse ? () => browse.more() : undefined}
    onjump={browse ? (column, value) => workspace.jumpTo(column, value) : undefined}
    references={browse ? workspace.references : {}}
    more={browse ? !browse.end : false}
    paging={browse?.paging ?? false}
    labels={{
      copyCell: m.menu_copy(),
      copyRow: m.menu_copy_row(),
      copyColumn: m.menu_copy_name(),
      copyAll: m.menu_copy_all(),
      inspect: m.menu_inspect(),
      jumpTo: m.menu_jump_to(),
      filterBy: m.menu_filter_by(),
      clearFilters: m.menu_clear_filters(),
      contains: m.filter_contains(),
      apply: m.apply(),
      discard: m.discard(),
      edited: m.edited(),
      noKey: m.no_key(),
      value: m.filter_value(),
      loading: m.loading_rows(),
    }}
  />
{/if}
