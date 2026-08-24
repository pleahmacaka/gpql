<script lang="ts">
  import { DataGrid } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"

  import { workspace } from "$lib/session/workspace.svelte"
  import type { QueryResult } from "$lib/types"

  type Props = { result: QueryResult | null; empty: string }

  let { result, empty }: Props = $props()
</script>

{#if !result || result.columns.length === 0}
  <p class="px-4 py-3 text-sm text-base-content/40">
    {result?.affected != null ? `${result.affected} rows touched` : empty}
  </p>
{:else}
  <DataGrid
    columns={result.columns}
    rows={result.rows}
    rowHeight={workspace.rowHeight}
    labels={{
      copyCell: m.menu_copy(),
      copyRow: m.menu_copy_row(),
      copyColumn: m.menu_copy_name(),
      filterBy: m.menu_filter_by(),
      clearFilters: m.menu_clear_filters(),
      contains: m.filter_contains(),
    }}
  />
{/if}
