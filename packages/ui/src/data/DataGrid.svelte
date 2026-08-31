<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual"
  import { untrack } from "svelte"

  import Dropdown from "../controls/Dropdown.svelte"
  import { drag } from "../controls/drag"
  import { menu, type MenuItem } from "../controls/menu.svelte"
  import { tooltip } from "../controls/tooltip"
  import { Icon } from "../icons"
  import { rem } from "../controls/rem"

  export type CellEdit = {
    keys: Record<string, string | null>
    set: Record<string, string | null>
  }

  type Filter = { op: string; value: string }
  export type Sort = { column: string; dir: "asc" | "desc" }

  type Props = {
    columns: string[]
    rows: (string | null)[][]
    types?: Record<string, string>
    rowHeight?: number
    filterable?: boolean
    editable?: boolean
    keyColumns?: string[]
    busy?: boolean
    minimap?: boolean
    spot?: { row: number; column: number } | null
    needle?: string
    onapply?: (edits: CellEdit[]) => Promise<void> | void
    onblocked?: () => void
    onsort?: (sort: Sort | null) => void
    onfilter?: (filters: Record<string, Filter>) => void
    onmore?: () => void
    onjump?: (column: string, value: string) => void
    references?: Record<string, string>
    paging?: boolean
    more?: boolean
    labels?: Partial<
      Record<
        | "copyCell"
        | "copyRow"
        | "copyColumn"
        | "filterBy"
        | "clearFilters"
        | "contains"
        | "apply"
        | "discard"
        | "edited"
        | "copyAll"
        | "inspect"
        | "jumpTo"
        | "noKey"
        | "value"
        | "loading",
        string
      >
    >
  }

  let {
    columns,
    rows,
    types = {},
    rowHeight = rem(2),
    filterable = true,
    editable = false,
    keyColumns = [],
    busy = false,
    minimap = true,
    spot = null,
    needle = "",
    onapply,
    onblocked,
    onsort,
    onfilter,
    onmore,
    onjump,
    references = {},
    paging = false,
    more = false,
    labels = {},
  }: Props = $props()

  // when the parent slices server-side it owns ordering and filtering, so
  // ranking the loaded page here would answer a different question
  let remote = $derived(!!onsort || !!onfilter)

  let words = $derived({
    copyCell: labels.copyCell ?? "Copy cell",
    copyRow: labels.copyRow ?? "Copy row",
    copyColumn: labels.copyColumn ?? "Copy column name",
    copyAll: labels.copyAll ?? "Copy loaded rows",
    inspect: labels.inspect ?? "Open value",
    jumpTo: labels.jumpTo ?? "Go to",
    filterBy: labels.filterBy ?? "Filter by this value",
    clearFilters: labels.clearFilters ?? "Clear filters",
    contains: labels.contains ?? "contains",
    apply: labels.apply ?? "Apply",
    discard: labels.discard ?? "Discard",
    edited: labels.edited ?? "edited",
    noKey: labels.noKey ?? "this result has no primary key to write back to",
    value: labels.value ?? "value",
    loading: labels.loading ?? "reading rows",
  })

  const DIRECTIONS = ["asc", "desc"] as const

  const FILTER_DELAY = 250

  const PAGE_MARGIN = 24

  const OPERATORS = [
    { id: "contains", label: "contains", needsValue: true },
    { id: "eq", label: "=", needsValue: true },
    { id: "ne", label: "≠", needsValue: true },
    { id: "gt", label: ">", needsValue: true },
    { id: "gte", label: "≥", needsValue: true },
    { id: "lt", label: "<", needsValue: true },
    { id: "lte", label: "≤", needsValue: true },
    { id: "starts", label: "starts with", needsValue: true },
    { id: "ends", label: "ends with", needsValue: true },
    { id: "isnull", label: "is null", needsValue: false },
    { id: "notnull", label: "is not null", needsValue: false },
  ]

  const DEFAULT_WIDTH = 176
  const MIN_WIDTH = rem(4.5)

  let viewport = $state<HTMLDivElement | null>(null)
  let widths = $state<Record<string, number>>({})
  let filters = $state<Record<string, Filter>>({})
  let sort = $state<Sort | null>(null)
  let openFilter = $state<string | null>(null)
  let cursor = $state<{ row: number; column: number } | null>(null)
  let editing = $state<{ row: number; column: number; draft: string } | null>(
    null,
  )
  let staged = $state<Record<string, string | null>>({})
  let detail = $state<{ column: string; value: string } | null>(null)


  let active = $derived(
    Object.entries(filters).filter(
      ([, filter]) => filter.value !== "" || !needsValue(filter.op),
    ),
  )

  let reported = $state(false)

  $effect(() => {
    const next = sort

    if (!untrack(() => reported)) {
      return
    }

    untrack(() => onsort?.(next ? { ...next } : null))
  })

  $effect(() => {
    const next = $state.snapshot(filters) as Record<string, Filter>

    if (!untrack(() => reported)) {
      return
    }

    const timer = setTimeout(() => onfilter?.(next), FILTER_DELAY)

    return () => clearTimeout(timer)
  })

  $effect(() => {
    reported = true
  })

  let dirty = $derived(Object.keys(staged).length)
  let writable = $derived(editable && keyColumns.length > 0)

  let shown = $derived.by(() => {
    if (remote) {
      return rows
    }

    const active = Object.entries(filters).filter(
      ([, filter]) => filter.value !== "" || !needsValue(filter.op),
    )

    let result =
      active.length === 0
        ? rows
        : rows.filter(row =>
            active.every(([name, filter]) =>
              matches(row[columns.indexOf(name)], filter),
            ),
          )

    if (sort) {
      const index = columns.indexOf(sort.column)
      const flip = sort.dir === "asc" ? 1 : -1

      result = [...result].sort(
        (a, b) => compare(a[index] ?? null, b[index] ?? null) * flip,
      )
    }

    return result
  })

  function compare(a: string | null, b: string | null) {
    if (a === null || b === null) {
      return a === b ? 0 : a === null ? 1 : -1
    }

    const numbers = [Number(a), Number(b)]

    if (numbers.every(entry => !Number.isNaN(entry))) {
      return numbers[0] - numbers[1]
    }

    return a.localeCompare(b)
  }


  function toggleSort(name: string) {
    if (sort?.column !== name) {
      sort = { column: name, dir: "asc" }
    } else if (sort.dir === "asc") {
      sort = { column: name, dir: "desc" }
    } else {
      sort = null
    }
  }

  const needsValue = (op: string) =>
    OPERATORS.find(entry => entry.id === op)?.needsValue ?? true

  const stamp = (row: number, column: number) => `${row}:${columns[column]}`

  function matches(cell: string | null | undefined, filter: Filter) {
    const value = cell ?? null

    if (filter.op === "isnull") {
      return value === null
    }

    if (filter.op === "notnull") {
      return value !== null
    }

    if (value === null) {
      return false
    }

    const left = value.toLowerCase()
    const right = filter.value.toLowerCase()
    const numbers = [Number(value), Number(filter.value)]
    const comparable = numbers.every(entry => !Number.isNaN(entry))

    switch (filter.op) {
      case "eq":
        return comparable ? numbers[0] === numbers[1] : left === right
      case "ne":
        return comparable ? numbers[0] !== numbers[1] : left !== right
      case "gt":
        return comparable ? numbers[0] > numbers[1] : left > right
      case "gte":
        return comparable ? numbers[0] >= numbers[1] : left >= right
      case "lt":
        return comparable ? numbers[0] < numbers[1] : left < right
      case "lte":
        return comparable ? numbers[0] <= numbers[1] : left <= right
      case "starts":
        return left.startsWith(right)
      case "ends":
        return left.endsWith(right)
      default:
        return left.includes(right)
    }
  }

  function cellOf(row: number, column: number) {
    const key = stamp(row, column)

    if (key in staged) {
      return staged[key]
    }

    return shown[row]?.[column] ?? null
  }

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

  let look = $state({ top: 0, left: 0, width: 1, height: 1 })
  let dragging = $state(false)

  let span = $derived({
    width: Math.max($columnScroller.getTotalSize(), 1),
    height: Math.max($rowScroller.getTotalSize(), 1),
  })

  let roams = $derived(
    minimap &&
      (span.width > look.width + 4 || span.height > look.height + 4) &&
      shown.length > 0,
  )

  let frame = $derived.by(() => {
    const width = Math.min(Math.max((look.width / span.width) * 100, 8), 100)
    const height = Math.min(Math.max((look.height / span.height) * 100, 8), 100)

    return {
      width,
      height,
      left: Math.min((look.left / span.width) * 100, 100 - width),
      top: Math.min((look.top / span.height) * 100, 100 - height),
    }
  })

  function watch() {
    if (!viewport) {
      return
    }

    look = {
      top: viewport.scrollTop,
      left: viewport.scrollLeft,
      width: viewport.clientWidth,
      height: viewport.clientHeight,
    }
  }

  function roam(event: PointerEvent) {
    const map = event.currentTarget as HTMLElement

    const walk = (moved: PointerEvent) => {
      if (!viewport) {
        return
      }

      const box = map.getBoundingClientRect()
      const across = (moved.clientX - box.left) / box.width
      const down = (moved.clientY - box.top) / box.height

      viewport.scrollTo({
        left: across * span.width - look.width / 2,
        top: down * span.height - look.height / 2,
        behavior: "auto",
      })
    }

    dragging = true
    walk(event)
    drag(event, walk, () => (dragging = false))
  }

  $effect(() => {
    const element = viewport

    if (!element) {
      return
    }

    watch()

    const watcher = new ResizeObserver(watch)

    watcher.observe(element)

    return () => watcher.disconnect()
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
    const items = $rowScroller.getVirtualItems()
    const last = items[items.length - 1]

    if (!last || !more || paging || !onmore) {
      return
    }

    if (last.index >= shown.length - PAGE_MARGIN) {
      untrack(() => onmore())
    }
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

  $effect(() => {
    void rows
    void columns
    void sort

    untrack(() => {
      staged = {}
      editing = null
      cursor = null
    })
  })

  function autoFit(name: string) {
    if (!viewport) {
      return
    }

    const context = document.createElement("canvas").getContext("2d")

    if (!context) {
      return
    }

    const style = getComputedStyle(viewport)

    context.font = `${style.fontSize} ${style.fontFamily}`

    const index = columns.indexOf(name)
    let widest = context.measureText(name).width + 24

    // ponytail: first 1000 rows only, full scan if wide tails matter
    for (const row of shown.slice(0, 1000)) {
      widest = Math.max(widest, context.measureText(row[index] ?? "null").width)
    }

    widths = {
      ...widths,
      [name]: Math.min(Math.max(Math.ceil(widest) + rem(2), MIN_WIDTH), rem(30)),
    }
  }

  function startResize(event: PointerEvent, name: string) {
    event.stopPropagation()

    const startX = event.clientX
    const startWidth = widthOf(name)

    drag(event, moved => {
      widths = {
        ...widths,
        [name]: Math.max(MIN_WIDTH, startWidth + moved.clientX - startX),
      }
    })
  }

  const TAB = "\t"

  function delimited() {
    const line = (cells: (string | null)[]) =>
      cells.map(cell => cell ?? "null").join(TAB)

    return [line(columns), ...shown.map(line)].join("\n")
  }

  // json and xml are the values worth widening a panel for, so lay them out
  function pretty(value: string) {
    const trimmed = value.trim()

    if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
      return value
    }

    try {
      return JSON.stringify(JSON.parse(trimmed), null, 2)
    } catch {
      return value
    }
  }

  function copy(text: string) {
    navigator.clipboard.writeText(text)
  }

  function openMenu(event: MouseEvent, row: number, column: number) {
    event.preventDefault()
    event.stopPropagation()

    cursor = { row, column }

    const name = columns[column]
    const cell = cellOf(row, column) ?? ""

    const items: MenuItem[] = [
      { label: words.copyCell, icon: "lucide:copy", run: () => copy(cell) },
      {
        label: words.copyRow,
        icon: "lucide:rows-3",
        run: () =>
          copy(
            columns
              .map((_, index) => cellOf(row, index) ?? "null")
              .join("\t"),
          ),
      },
      {
        label: words.copyColumn,
        icon: "lucide:columns-3",
        run: () => copy(name),
      },
      {
        label: words.copyAll,
        icon: "lucide:clipboard-list",
        run: () => copy(delimited()),
      },
      {
        label: words.inspect,
        icon: "lucide:maximize-2",
        run: () => (detail = { column: name, value: cell }),
      },
      ...(onjump && references[name] && cell !== "null"
        ? [
            {
              label: `${words.jumpTo} ${references[name]}`,
              icon: "lucide:arrow-right-to-line",
              run: () => onjump(name, cell),
            },
          ]
        : []),
      {
        label: words.filterBy,
        icon: "lucide:filter",
        run: () => {
          filters = { ...filters, [name]: { op: "eq", value: cell } }
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
    ]

    menu.show(event, items)
  }

  function wheel(event: WheelEvent) {
    if (!viewport || event.deltaX !== 0) {
      return
    }

    const sideways = event.shiftKey || viewport.scrollHeight <= viewport.clientHeight

    if (!sideways) {
      return
    }

    event.preventDefault()
    viewport.scrollLeft += event.deltaY
  }

  function labelOf(op: string) {
    return OPERATORS.find(entry => entry.id === op)?.label ?? op
  }

  function headerTip(name: string) {
    const lines = [name]

    if (types[name]) {
      lines.push(types[name])
    }

    const filter = filters[name]

    if (filter && (filter.value !== "" || !needsValue(filter.op))) {
      lines.push(
        `${labelOf(filter.op)} ${needsValue(filter.op) ? filter.value : ""}`.trim(),
      )
    }

    if (sort?.column === name) {
      lines.push(sort.dir === "asc" ? "↑ asc" : "↓ desc")
    }

    return lines.join("\n")
  }

  let tips = $derived(
    Object.fromEntries(columns.map(name => [name, headerTip(name)])),
  )

  function activeOn(name: string) {
    const filter = filters[name]

    return !!filter && (filter.value !== "" || !needsValue(filter.op))
  }

  function closeFilter() {
    if (openFilter && !activeOn(openFilter)) {
      const next = { ...filters }

      delete next[openFilter]
      filters = next
    }

    openFilter = null
  }

  function toggleFilter(name: string) {
    if (!filterable) {
      return
    }

    const closing = openFilter === name

    closeFilter()

    if (!closing) {
      if (!filters[name]) {
        filters = { ...filters, [name]: { op: "contains", value: "" } }
      }

      openFilter = name
    }
  }

  function setFilter(name: string, patch: Partial<Filter>) {
    filters = {
      ...filters,
      [name]: { ...(filters[name] ?? { op: "contains", value: "" }), ...patch },
    }
  }

  function dropFilter(name: string) {
    const next = { ...filters }

    delete next[name]
    filters = next
    openFilter = null
  }

  // a find hit outside the grid still has to bring the cell into view
  $effect(() => {
    const target = spot

    if (!target) {
      return
    }

    untrack(() => {
      const index = shown.indexOf(rows[target.row])

      if (index >= 0) {
        focusCell(index, target.column)
      }
    })
  })

  function focusCell(row: number, column: number) {
    const bounded = {
      row: Math.max(0, Math.min(row, shown.length - 1)),
      column: Math.max(0, Math.min(column, columns.length - 1)),
    }

    cursor = bounded
    $rowScroller.scrollToIndex(bounded.row, {
      align: "auto",
      behavior: "smooth",
    })
    $columnScroller.scrollToIndex(bounded.column, {
      align: "auto",
      behavior: "smooth",
    })
  }

  function beginEdit(seed?: string) {
    if (!cursor) {
      return
    }

    if (!editable) {
      onblocked?.()

      return
    }

    if (!writable) {
      return
    }

    editing = {
      ...cursor,
      draft: seed ?? cellOf(cursor.row, cursor.column) ?? "",
    }
  }

  function commitEdit(send = false) {
    if (!editing) {
      return
    }

    const key = stamp(editing.row, editing.column)
    const original = shown[editing.row]?.[editing.column] ?? null
    const next = editing.draft

    if (next === (original ?? "")) {
      const rest = { ...staged }

      delete rest[key]
      staged = rest
    } else {
      staged = { ...staged, [key]: next }
    }

    editing = null
    viewport?.focus()

    if (send) {
      void apply()
    }
  }

  function clearCell() {
    if (!writable || !cursor) {
      return
    }

    staged = { ...staged, [stamp(cursor.row, cursor.column)]: null }
  }

  function discard() {
    staged = {}
    editing = null
  }

  async function apply() {
    if (!onapply || dirty === 0) {
      return
    }

    const byRow = new Map<number, Record<string, string | null>>()

    for (const [key, value] of Object.entries(staged)) {
      const [row, column] = [
        Number(key.slice(0, key.indexOf(":"))),
        key.slice(key.indexOf(":") + 1),
      ]
      const patch = byRow.get(row) ?? {}

      patch[column] = value
      byRow.set(row, patch)
    }

    const edits: CellEdit[] = [...byRow.entries()].map(([row, set]) => ({
      keys: Object.fromEntries(
        keyColumns.map(name => [name, shown[row]?.[columns.indexOf(name)] ?? null]),
      ),
      set,
    }))

    await onapply(edits)
    staged = {}
  }

  function keys(event: KeyboardEvent) {
    if (editing) {
      return
    }

    if (!cursor) {
      if (event.key.startsWith("Arrow")) {
        event.preventDefault()
        focusCell(0, 0)
      }

      return
    }

    const moves: Record<string, [number, number]> = {
      ArrowDown: [1, 0],
      ArrowUp: [-1, 0],
      ArrowRight: [0, 1],
      ArrowLeft: [0, -1],
      Tab: [0, event.shiftKey ? -1 : 1],
      Enter: [1, 0],
      PageDown: [12, 0],
      PageUp: [-12, 0],
    }

    if (event.key === "Enter" && writable && !event.ctrlKey) {
      event.preventDefault()
      beginEdit()

      return
    }

    if (event.key === "F2") {
      event.preventDefault()
      beginEdit()

      return
    }

    if (event.key === "Home") {
      event.preventDefault()
      focusCell(event.ctrlKey ? 0 : cursor.row, 0)

      return
    }

    if (event.key === "End") {
      event.preventDefault()
      focusCell(
        event.ctrlKey ? shown.length - 1 : cursor.row,
        columns.length - 1,
      )

      return
    }

    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault()
      clearCell()

      return
    }

    if (event.key === "c" && event.ctrlKey) {
      event.preventDefault()
      copy(cellOf(cursor.row, cursor.column) ?? "null")

      return
    }

    const step = moves[event.key]

    if (step) {
      event.preventDefault()
      focusCell(cursor.row + step[0], cursor.column + step[1])

      return
    }

    if (
      writable &&
      event.key.length === 1 &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.metaKey
    ) {
      event.preventDefault()
      beginEdit(event.key)
    }
  }
</script>

<div class="relative flex min-h-0 min-w-0 flex-1 flex-col">
  {#if active.length > 0}
    <div
      class="absolute bottom-3 left-1/2 z-30 flex max-w-full -translate-x-1/2
        flex-wrap items-center gap-1 rounded-field floating px-2 py-1 lift"
    >
      {#each active as [name, filter] (name)}
        <span
          class="flex items-center gap-1 rounded-selector bg-primary/10 pr-1
            pl-2 text-xs text-primary"
        >
          <button
            type="button"
            onclick={() => toggleFilter(name)}
            class="flex items-center gap-1 py-1"
          >
            <span class="font-medium">{name}</span>
            <span class="opacity-70">{labelOf(filter.op)}</span>

            {#if needsValue(filter.op)}
              <span class="max-w-24 truncate">{filter.value}</span>
            {/if}
          </button>

          <button
            type="button"
            aria-label="drop {name}"
            onclick={() => dropFilter(name)}
            class="rounded-selector p-1 opacity-60 hover:opacity-100"
          >
            <Icon icon="lucide:x" class="size-3" />
          </button>
        </span>
      {/each}

      <button
        type="button"
        onclick={() => {
          filters = {}
          openFilter = null
        }}
        class="rounded-selector px-2 py-1 text-xs text-base-content/45
          hover:text-error"
      >
        {words.clearFilters}
      </button>
    </div>
  {/if}

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    bind:this={viewport}
    role="grid"
  tabindex="0"
  onkeydown={keys}
  onwheel={wheel}
  onscroll={watch}
  style:scroll-behavior={dragging ? "auto" : "smooth"}
  oncontextmenu={event => event.preventDefault()}
  class="relative min-h-0 flex-1 overflow-auto outline-none
    select-none"
>
  <div
    class="sticky top-0 z-20 floating"
    style:width="{$columnScroller.getTotalSize()}px"
  >
    <div class="relative h-8">
      {#each $columnScroller.getVirtualItems() as column (column.key)}
        {@const name = columns[column.index]}

        <div
          class="absolute top-0 flex h-8 items-center"
          style:left="{column.start}px"
          style:width="{column.size}px"
        >
          <button
            type="button"
            onclick={() => toggleFilter(name)}
            use:tooltip={tips[name]}
            class="flex h-full min-w-0 flex-1 items-center gap-1 px-4 text-left
              text-xs hover:text-base-content
              {activeOn(name) || sort?.column === name
                ? 'bg-primary/10 text-primary'
                : 'text-base-content/45'}"
          >
            <span class="truncate">{name}</span>

            {#if types[name]}
              <span class="truncate text-base-content/30 lowercase">
                {types[name]}
              </span>
            {/if}

            {#if keyColumns.includes(name)}
              <Icon icon="lucide:key-round" class="size-3 shrink-0 text-accent" />
            {/if}

            {#if activeOn(name)}
              <Icon icon="lucide:filter" class="size-3 shrink-0" />
            {/if}

          </button>

          <button
            type="button"
            aria-label="Sort {name}"
            onclick={() => toggleSort(name)}
            class="shrink-0 rounded-selector p-1
              {sort?.column === name
                ? 'text-primary'
                : 'text-base-content/25 hover:text-base-content/60'}"
          >
            <Icon
              icon={sort?.column === name
                ? sort.dir === "asc"
                  ? "lucide:arrow-up"
                  : "lucide:arrow-down"
                : "lucide:arrow-up-down"}
              class="size-3"
            />
          </button>

          <button
            type="button"
            aria-label="Resize {name}"
            onpointerdown={event => startResize(event, name)}
            ondblclick={() => autoFit(name)}
            class="h-8 w-1 shrink-0 cursor-col-resize bg-transparent
              hover:bg-primary/40"
          ></button>
        </div>
      {/each}
    </div>

    {#if openFilter}
      {@const anchor = $columnScroller
        .getVirtualItems()
        .find(column => columns[column.index] === openFilter)}
      {@const current = filters[openFilter] ?? { op: "contains", value: "" }}
      {@const target = openFilter}

      {#if anchor}
        <div
          class="absolute z-30 w-72 rounded-box floating p-3 lift"
          style:left="{Math.max(anchor.start - 8, 0)}px"
          style:top="2.25rem"
        >
          <div class="flex items-center gap-2 pb-2">
            <Icon icon="lucide:filter" class="size-4 text-base-content/40" />

            <span class="min-w-0 flex-1 truncate text-sm font-medium">
              {openFilter}
            </span>

            <button
              type="button"
              aria-label="close"
              onclick={closeFilter}
              class="rounded-selector p-1 text-base-content/35
                hover:text-base-content"
            >
              <Icon icon="lucide:x" class="size-4" />
            </button>
          </div>

          <div
            class="flex items-stretch rounded-field bg-base-200
              focus-within:ring-1 focus-within:ring-primary/40"
          >
            <div
              class="flex shrink-0 items-center py-1 pl-1 {needsValue(current.op)
                ? 'w-28 border-r border-base-content/8'
                : 'flex-1'}"
            >
              <Dropdown
                wide
                value={current.op}
                options={OPERATORS.map(entry => ({
                  value: entry.id,
                  label: entry.label,
                }))}
                onpick={op => setFilter(openFilter ?? "", { op })}
              />
            </div>

            {#if needsValue(current.op)}
              <!-- svelte-ignore a11y_autofocus -->
              <input
                autofocus
                value={current.value}
                oninput={event =>
                  setFilter(openFilter ?? "", {
                    value: event.currentTarget.value,
                  })}
                onkeydown={event => {
                  if (event.key === "Escape" || event.key === "Enter") {
                    event.stopPropagation()
                    closeFilter()
                  }
                }}
                placeholder={words.value}
                class="min-w-0 flex-1 bg-transparent px-2 py-2 text-sm
                  outline-none select-text placeholder:text-base-content/30"
              />
            {/if}
          </div>

          <div class="flex items-center gap-1 pt-2">
            <Icon
              icon="lucide:arrow-up-down"
              class="size-4 text-base-content/40"
            />

            {#each DIRECTIONS as dir (dir)}
              {@const chosen =
                sort?.column === openFilter && sort.dir === dir}

              <button
                type="button"
                aria-pressed={chosen}
                onclick={() => {
                  sort = chosen ? null : { column: target, dir }
                }}
                class="flex items-center gap-1 rounded-selector px-2 py-1
                  text-xs {chosen
                  ? 'bg-primary text-primary-content'
                  : 'bg-base-200 hover:bg-base-300'}"
              >
                <Icon
                  icon={dir === "asc" ? "lucide:arrow-up" : "lucide:arrow-down"}
                  class="size-3"
                />
                {dir}
              </button>
            {/each}
          </div>

          <div class="flex items-center justify-between pt-3">
            <button
              type="button"
              onclick={() => dropFilter(openFilter ?? "")}
              class="text-xs text-base-content/45 hover:text-error"
            >
              {words.clearFilters}
            </button>

            <button
              type="button"
              onclick={closeFilter}
              class="rounded-field bg-primary px-3 py-1 text-xs
                text-primary-content"
            >
              {words.apply}
            </button>
          </div>
        </div>
      {/if}
    {/if}
  </div>

  <div
    class="relative"
    style:height="{$rowScroller.getTotalSize()}px"
    style:width="{$columnScroller.getTotalSize()}px"
  >
    {#each $rowScroller.getVirtualItems() as row (row.key)}
      <div
        class="absolute inset-x-0 flex items-center contain-paint text-sm
          hover:bg-base-200/60"
        style:height="{row.size}px"
        style:transform="translateY({row.start}px)"
      >
        {#each $columnScroller.getVirtualItems() as column (column.key)}
          {@const cell = cellOf(row.index, column.index)}
          {@const touched = stamp(row.index, column.index) in staged}
          {@const here =
            cursor?.row === row.index && cursor?.column === column.index}
          {@const found =
            spot != null &&
            shown[row.index] === rows[spot.row] &&
            spot.column === column.index}
          {@const match =
            needle !== "" &&
            cell != null &&
            cell.toLowerCase().includes(needle)}

          {#if editing?.row === row.index && editing?.column === column.index}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              autofocus
              value={editing.draft}
              oninput={event => {
                if (editing) {
                  editing.draft = event.currentTarget.value
                }
              }}
              onblur={() => commitEdit()}
              onkeydown={event => {
                if (event.key === "Enter") {
                  event.preventDefault()
                  commitEdit(true)
                }

                if (event.key === "Escape") {
                  event.preventDefault()
                  event.stopPropagation()
                  editing = null
                  viewport?.focus()
                }
              }}
              class="absolute z-10 h-full bg-base-100 px-4 text-sm outline-none
                select-text ring-2 ring-primary ring-inset"
              style:left="{column.start}px"
              style:width="{column.size}px"
            />
          {:else}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <span
              onclick={() => focusCell(row.index, column.index)}
              ondblclick={() => {
                focusCell(row.index, column.index)
                beginEdit()
              }}
              oncontextmenu={event => openMenu(event, row.index, column.index)}
              class="absolute flex h-full items-center truncate px-4
                {cell === null ? 'text-base-content/35 italic' : ''}
                {found
                  ? 'bg-accent/30 ring-2 ring-accent ring-inset'
                  : match
                    ? 'bg-accent/15'
                    : touched
                      ? 'bg-primary/10 text-primary'
                      : ''}
                {here && !found ? 'ring-2 ring-primary/70 ring-inset' : ''}"
              style:left="{column.start}px"
              style:width="{column.size}px"
              title={cell ?? "null"}
            >
              {cell ?? "null"}
            </span>
          {/if}
        {/each}
      </div>
    {/each}
  </div>

</div>

  {#if detail}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      onclick={() => (detail = null)}
      class="absolute inset-0 z-50 grid place-items-center bg-base-300/45 p-6"
    >
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        onclick={event => event.stopPropagation()}
        class="flex max-h-full w-full max-w-2xl flex-col rounded-box floating lift"
      >
        <header class="flex items-center gap-2 px-4 pt-3 pb-2">
          <h3 class="min-w-0 flex-1 truncate text-sm">{detail.column}</h3>

          <button
            type="button"
            onclick={() => copy(detail?.value ?? "")}
            class="rounded-field bg-base-200 px-2 py-1 text-xs hover:bg-base-300"
          >
            {words.copyCell}
          </button>

          <button
            type="button"
            aria-label="close"
            onclick={() => (detail = null)}
            class="rounded-selector p-1 text-base-content/40
              hover:text-base-content"
          >
            <Icon icon="lucide:x" class="size-4" />
          </button>
        </header>

        <div class="min-h-0 flex-1 overflow-auto px-4 pb-4">
          <pre
            class="rounded-field bg-base-200 p-3 text-xs whitespace-pre-wrap
              select-text">{pretty(detail.value)}</pre>
        </div>
      </div>
    </div>
  {/if}

  {#if busy || paging}
    <div
      class="pointer-events-none absolute bottom-3 left-3 z-30 flex
        items-center gap-2 rounded-field floating px-2 py-1 text-xs
        text-base-content/60 lift"
    >
      <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
      {words.loading}
    </div>
  {/if}

  {#if roams}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      onpointerdown={roam}
      class="absolute right-3 bottom-3 z-30 h-20 w-32 cursor-crosshair
        overflow-hidden rounded-field floating lift"
    >
      {#each $columnScroller.getVirtualItems() as column (column.key)}
        <span
          class="absolute top-0 bottom-0 w-px bg-base-content/10"
          style:left="{(column.start / span.width) * 100}%"
        ></span>
      {/each}

      <span
        class="absolute rounded-field border border-primary/70 bg-primary/10"
        style:left="{frame.left}%"
        style:top="{frame.top}%"
        style:width="{frame.width}%"
        style:height="{frame.height}%"
      ></span>
    </div>
  {/if}

  {#if dirty > 0}
    <div
      class="flex items-center gap-3 border-t border-base-content/8 px-4 py-2
        text-sm"
    >
      <Icon icon="lucide:pencil" class="size-4 text-primary" />

      <span class="flex-1 text-xs text-base-content/60">
        {dirty}
        {words.edited}
      </span>

      <button
        type="button"
        onclick={discard}
        class="rounded-field px-3 py-1 text-xs hover:bg-base-200"
      >
        {words.discard}
      </button>

      <button
        type="button"
        onclick={apply}
        disabled={busy}
        class="rounded-field bg-primary px-3 py-1 text-xs text-primary-content
          disabled:opacity-50"
      >
        {words.apply}
      </button>
    </div>
  {:else if editable && keyColumns.length === 0}
    <p class="px-4 py-2 text-xs text-base-content/40">{words.noKey}</p>
  {/if}
</div>
