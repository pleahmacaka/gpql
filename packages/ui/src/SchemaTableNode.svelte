<script lang="ts">
  import { Handle, Position, type NodeProps } from "@xyflow/svelte"

  import { board } from "./board.svelte"
  import { Icon } from "./icons"
  import type { SchemaTable } from "./types"

  let { data }: NodeProps = $props()

  let table = $derived(data.table as SchemaTable)
  let active = $derived(board.selected === table.name)
  let focused = $derived(board.on(table.name))
</script>

<div
  class="w-72 rounded-box p-3 transition-colors
    {active ? 'bg-primary/10' : 'bg-base-200/70 hover:bg-base-200'}
    {focused ? 'ring-1 ring-primary' : 'hairline'}"
>
  <Handle
    type="target"
    id="referenced"
    position={Position.Right}
    class="!size-1.5 !border-0 !bg-base-content/25"
    style="top: 1.35rem"
  />

  <header class="flex items-center gap-2 pb-2">
    <Icon
      icon="lucide:table-2"
      class="size-3.5 {active ? 'text-primary' : 'text-base-content/40'}"
    />

    <h3 class="flex-1 truncate text-sm {active ? 'text-primary' : ''}">
      {table.name}
    </h3>

    <span class="text-xs text-base-content/40">{table.rows}</span>
  </header>

  {#if table.note}
    <p class="pb-2 text-xs text-base-content/50 italic">{table.note}</p>

    <Handle
      type="source"
      id="note"
      position={Position.Left}
      class="!size-1.5 !border-0 !bg-base-content/30"
      style="top: 2.6rem"
    />
  {/if}

  <dl>
    {#each table.columns as column, index (column.name)}
      <div
        class="relative flex h-5.5 items-center gap-1.5 rounded-selector
          {board.at(table.name, index) ? 'bg-primary/15' : ''}"
      >
        {#if column.references}
          <Handle
            type="source"
            id={column.name}
            position={Position.Left}
            class="!size-1.5 !border-0 !bg-primary"
          />
        {/if}

        {#if column.primaryKey}
          <Icon icon="lucide:key-round" class="size-3 shrink-0 text-accent" />
        {:else}
          <span class="size-3 shrink-0"></span>
        {/if}

        <dt class="truncate text-xs" title={column.note ?? undefined}>
          {column.name}
        </dt>

        {#if column.required}
          <span class="text-xs text-base-content/35">*</span>
        {/if}

        <dd class="flex-1 truncate text-right text-xs text-base-content/40">
          {#if column.references}
            <span
              class="rounded-selector bg-primary/10 px-1.5 py-0.5 text-primary"
            >
              → {column.references}
            </span>
          {:else}
            {column.dataType}
          {/if}
        </dd>
      </div>
    {/each}
  </dl>
</div>
