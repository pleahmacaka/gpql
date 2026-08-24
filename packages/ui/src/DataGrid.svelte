<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual"
  import { untrack } from "svelte"

  import ContextMenu from "./ContextMenu.svelte"
  import { Icon } from "./icons"

  type Props = {
    columns: string[]
    rows: (string | null)[][]
    rowHeight?: number
    filterable?: boolean
    labels?: Partial<
      Record<
        "copyCell" | "copyRow" | "copyColumn" | "filterBy" | "clearFilters" | "contains",
        string
      >
    >
  }

  let {
    columns,
    rows,
    rowHeight = 34,
    filterable = true,
    labels = {},
  }: Props = $props()

  let words = $derived({
    copyCell: labels.copyCell ?? "Copy cell",
    copyRow: labels.copyRow ?? "Copy row",
    copyColumn: labels.copyColumn ?? "Copy column name",
    filterBy: labels.filterBy ?? "Filter by this value",
    clearFilters: labels.clearFilters ?? "Clear filters",
    contains: labels.contains ?? "contains",
  })

  const DEFAULT_WIDTH = 176
  const MIN_WIDTH = 72

  let viewport = $state<HTMLDivElement | null>(null)
  let widths = $state<Record<string, number>>({})
  let filters = $state<Record<string, string>>({})
  let openFilter = $state<string | null>(null)
  let menu = $state<{
    x: number
    y: number
    items: { label: string; icon?: string; danger?: boolean; run: () => void }[]
  } | null>(null)

  let shown = $derived.by(() => {
    const active = Object.entries(filters).filter(([, term]) => term !== "")

    if (active.length === 0) {
      return rows
    }

    return rows.filter(row =>
      active.every(([name, term]) => {
        const cell = row[columns.indexOf(name)]

        return (cell ?? "null").toLowerCase().includes(term.toLowerCase())
      }),
    )
  })

  const widthOf = (name: string) => widths[name] ?? DEFAULT_WIDTH

  const rowScroller = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => viewport,
    estimateSize: () => rowHeight,
    overscan: 12,
  })

  const columnScroller = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    horizontal: true,
    count: 0,
    getScrollElement: () => viewport,
    estimateSize: () => DEFAULT_WIDTH,
    overscan: 4,
  })

  $effect(() => {
    const count = shown.length
    const height = rowHeight
    const element = viewport

    untrack(() => {
      $rowScroller.setOptions({
        count,
        estimateSize: () => height,
        getScrollElement: () => element,
      })
      $rowScroller.measure()
    })
  })

  $effect(() => {
    const names = columns
    const sizes = { ...widths }
    const element = viewport

    untrack(() => {
      $columnScroller.setOptions({
        count: names.length,
        estimateSize: index => sizes[names[index]] ?? DEFAULT_WIDTH,
        getScrollElement: () => element,
      })
      $columnScroller.measure()
    })
  })

  function startResize(event: PointerEvent, name: string) {
    event.preventDefault()
    event.stopPropagation()

    const startX = event.clientX
    const startWidth = widthOf(name)

    const move = (moved: PointerEvent) => {
      widths = {
        ...widths,
        [name]: Math.max(MIN_WIDTH, startWidth + moved.clientX - startX),
      }
    }

    const done = () => {
      window.removeEventListener("pointermove", move)
      window.removeEventListener("pointerup", done)
    }

    window.addEventListener("pointermove", move)
    window.addEventListener("pointerup", done)
  }

  function copy(text: string) {
    navigator.clipboard.writeText(text)
  }

  function openMenu(event: MouseEvent, row: number, column: number) {
    event.preventDefault()

    const name = columns[column]
    const cell = shown[row]?.[column] ?? ""

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        { label: words.copyCell, icon: "lucide:copy", run: () => copy(cell) },
        {
          label: words.copyRow,
          icon: "lucide:rows-3",
          run: () =>
            copy((shown[row] ?? []).map(value => value ?? "null").join("	")),
        },
        {
          label: words.copyColumn,
          icon: "lucide:columns-3",
          run: () => copy(name),
        },
        {
          label: words.filterBy,
          icon: "lucide:filter",
          run: () => {
            filters = { ...filters, [name]: cell }
          },
        },
        {
          label: words.clearFilters,
          icon: "lucide:filter-x",
          danger: true,
          run: () => {
            filters = {}
            openFilter = null
          },
        },
      ],
    }
  }

  function toggleFilter(name: string) {
    if (!filterable) {
      return
    }

    openFilter = openFilter === name ? null : name
  }
</script>

<div bind:this={viewport} class="relative min-h-0 flex-1 overflow-auto">
  <div
    class="sticky top-0 z-20 bg-base-100"
    style:width="{$columnScroller.getTotalSize()}px"
  >
    <div class="relative h-6">
      {#each $columnScroller.getVirtualItems() as column (column.key)}
        {@const name = columns[column.index]}

        <div
          class="absolute top-0 flex h-6 items-center"
          style:left="{column.start}px"
          style:width="{column.size}px"
        >
          <button
            type="button"
            onclick={() => toggleFilter(name)}
            class="flex min-w-0 flex-1 items-center gap-1 px-4 text-left text-xs
              hover:text-base-content
              {filters[name] ? 'text-primary' : 'text-base-content/45'}"
          >
            <span class="truncate">{name}</span>

            {#if filters[name]}
              <Icon icon="lucide:filter" class="size-3 shrink-0" />
            {/if}
          </button>

          <button
            type="button"
            aria-label="Resize {name}"
            onpointerdown={event => startResize(event, name)}
            class="h-6 w-1 shrink-0 cursor-col-resize bg-transparent
              hover:bg-primary/40"
          ></button>
        </div>
      {/each}
    </div>

    {#if openFilter}
      <div class="relative h-7">
        {#each $columnScroller.getVirtualItems() as column (column.key)}
          {#if columns[column.index] === openFilter}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              autofocus
              value={filters[openFilter] ?? ""}
              oninput={event => {
                filters = {
                  ...filters,
                  [openFilter ?? ""]: event.currentTarget.value,
                }
              }}
              onkeydown={event => {
                if (event.key === "Escape") {
                  openFilter = null
                }
              }}
              placeholder={words.contains}
              class="absolute top-0 h-6 rounded-field bg-base-200 px-2 text-xs
                outline-none placeholder:text-base-content/30"
              style:left="{column.start + 12}px"
              style:width="{Math.max(column.size - 24, 48)}px"
            />
          {/if}
        {/each}
      </div>
    {/if}
  </div>

  <div
    class="relative"
    style:height="{$rowScroller.getTotalSize()}px"
    style:width="{$columnScroller.getTotalSize()}px"
  >
    {#each $rowScroller.getVirtualItems() as row (row.key)}
      <div
        class="absolute inset-x-0 flex items-center text-sm hover:bg-base-200/60"
        style:height="{row.size}px"
        style:transform="translateY({row.start}px)"
      >
        {#each $columnScroller.getVirtualItems() as column (column.key)}
          {@const cell = shown[row.index][column.index] ?? null}

          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span
            oncontextmenu={event => openMenu(event, row.index, column.index)}
            class="absolute truncate px-4 {cell === null
              ? 'text-base-content/35 italic'
              : ''}"
            style:left="{column.start}px"
            style:width="{column.size}px"
            title={cell ?? "null"}
          >
            {cell ?? "null"}
          </span>
        {/each}
      </div>
    {/each}
  </div>

  {#if menu}
    <ContextMenu
      x={menu.x}
      y={menu.y}
      items={menu.items}
      onclose={() => (menu = null)}
    />
  {/if}
</div>
