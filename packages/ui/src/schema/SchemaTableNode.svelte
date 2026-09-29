<script lang="ts">
  import { Handle, type NodeProps, Position } from "@xyflow/svelte"

  import { tooltip } from "../controls/tooltip"
  import { Icon } from "../icons"
  import type { SchemaColumn, SchemaTable } from "../types"
  import { board, spoken } from "./board.svelte"
  import { CARD, noteHeight, noteRows } from "./levels"

  let { data, selected }: NodeProps = $props()

  const words = spoken()
  const count = new Intl.NumberFormat()

  let table = $derived(data.table as SchemaTable)
  let active = $derived(board.selected === table.name)
  let focused = $derived(board.on(table.name))
  let rows = $derived(noteRows(table))
  let hinted = $derived((table.hints?.length ?? 0) > 0)

  let noteAt = $derived(
    rows === 0 ? CARD.header / 2 : CARD.header + noteHeight(table) / 2,
  )

  function describe(column: SchemaColumn) {
    const said = words()
    const shape = [
      column.dataType,
      column.primaryKey && said.primary,
      !column.required && !column.primaryKey && said.nullable,
    ]

    return [
      column.name,
      shape.filter(Boolean).join(", "),
      column.references && `${said.references} ${column.references}`,
      column.note,
    ]
      .filter(Boolean)
      .join("\n")
  }
</script>

<article
  aria-label={table.name}
  class={[
    "group/card relative w-72 bg-base-100 hairline",
    selected && "outline-1 outline-primary",
  ]}
>
  <Handle
    type="target"
    id="referenced"
    position={Position.Right}
    class="size-2! rounded-none! border-0! bg-base-content/40!"
    style="top: {CARD.header / 2}rem"
  />

  {#if hinted}
    <Handle
      type="source"
      id="note"
      position={Position.Left}
      class="size-2! rounded-none! border-0! bg-info!"
      style="top: {noteAt}rem"
    />
  {/if}

  <header
    class={[
      "flex h-10 items-center gap-2 border-b border-base-content/10 px-3",
      "transition-colors",
      active && "bg-primary/10",
    ]}
  >
    <Icon
      icon="lucide:table-2"
      class={[
        "size-4 shrink-0",
        active ? "text-primary" : "text-base-content/70",
      ]}
    />

    <h3
      use:tooltip={table.name}
      class={[
        "min-w-0 flex-1 truncate text-sm font-semibold",
        active && "text-primary",
      ]}
    >
      {table.name}
    </h3>

    {#if table.rows > 0}
      <span class="badge badge-sm badge-ghost shrink-0 tabular-nums">
        {count.format(table.rows)}
      </span>
    {/if}

    {#if board.onopen}
      <button
        type="button"
        tabindex="-1"
        aria-label="{words().open}, {table.name}"
        use:tooltip={words().open}
        onclick={event => {
          event.stopPropagation()
          board.onopen?.(table.name)
        }}
        class="nodrag btn btn-square btn-ghost btn-xs shrink-0"
      >
        <Icon icon="lucide:arrow-up-right" class="size-4" />
      </button>
    {/if}
  </header>

  {#if rows > 0}
    <p
      use:tooltip={table.note}
      class={[
        "border-b border-base-content/10 px-3 py-2 text-xs leading-4",
        "text-base-content/70",
        rows === 1 ? "line-clamp-1 h-8" : "line-clamp-2 h-12",
      ]}
    >
      {table.note}
    </p>
  {/if}

  {#each table.policies ?? [] as policy, index (index)}
    <p
      class={[
        "flex h-6 items-center gap-2 border-b border-base-content/10 px-3",
        "text-xs text-base-content/70",
      ]}
    >
      <Icon icon="lucide:shield" class="size-3 shrink-0" />
      <span class="truncate">{policy}</span>
    </p>
  {/each}

  <ul class="py-1">
    {#each table.columns as column, index (index)}
      {@const here = board.at(table.name, index)}
      {@const match =
        board.needle !== "" &&
        column.name.toLowerCase().includes(board.needle)}

      <li
        use:tooltip={describe(column)}
        class={[
          "relative flex h-6 items-center gap-2 px-3 text-xs",
          here ? "bg-primary/15" : match && "bg-accent/20",
        ]}
      >
        {#if here}
          <span
            aria-hidden="true"
            class="absolute inset-y-0 left-0 w-1 bg-primary"
          ></span>
        {/if}

        {#if column.references}
          <Handle
            type="source"
            id="column:{index}"
            position={Position.Left}
            class="size-2! rounded-none! border-0! bg-primary!"
          />
        {/if}

        {#if column.primaryKey}
          <span
            class={[
              "grid size-4 shrink-0 place-items-center",
              "bg-accent text-accent-content",
            ]}
          >
            <Icon icon="lucide:key-round" class="size-3" />
          </span>
        {:else if column.references}
          <span
            class={[
              "grid size-4 shrink-0 place-items-center",
              "bg-primary/15 text-primary",
            ]}
          >
            <Icon icon="lucide:link-2" class="size-3" />
          </span>
        {:else}
          <span aria-hidden="true" class="size-4 shrink-0"></span>
        {/if}

        <span
          class={[
            "min-w-0 flex-1 truncate",
            column.primaryKey && "font-semibold",
          ]}
        >
          {column.name}
        </span>

        <span
          class="max-w-28 shrink-0 truncate text-base-content/70 tabular-nums"
        >
          {column.dataType}{column.required || column.primaryKey ? "" : "?"}
        </span>
      </li>
    {/each}
  </ul>

  <span
    aria-hidden="true"
    class={[
      "hud hud-small pointer-events-none absolute inset-0 opacity-0",
      "transition-opacity group-hover/card:opacity-100",
      (focused || active) && "hud-lit opacity-100",
    ]}
  ></span>
</article>
