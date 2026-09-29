<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual"
  import { type Snippet, untrack } from "svelte"
  import { flip } from "svelte/animate"
  import { fade, scale } from "svelte/transition"

  import Dialog from "../controls/Dialog.svelte"
  import Dropdown from "../controls/Dropdown.svelte"
  import EmptyState from "../controls/EmptyState.svelte"
  import Keycap from "../controls/Keycap.svelte"
  import Segmented from "../controls/Segmented.svelte"
  import { drag } from "../controls/drag"
  import { menu, type MenuItem } from "../controls/menu.svelte"
  import { rem } from "../controls/rem"
  import { tooltip } from "../controls/tooltip"
  import { Icon } from "../icons"
  import { type Ending, leave, pop, rise, TIMING, veil } from "../motion"
  import { looksStructured, pretty as format, settle } from "./pretty"

  export type CellEdit = {
    keys: Record<string, string | null>
    set: Record<string, string | null>
  }

  export type Filter = { op: string; value: string }
  export type Sort = { column: string; dir: "asc" | "desc" }

  type Cell = { row: number; column: number }
  type Row = (string | null)[]
  type Order = { column: number; dir: "asc" | "desc" }

  type Staged = {
    column: number
    keys: Record<string, string | null>
    value: string | null
  }

  type Editing = Cell & {
    source: Row
    draft: string
    wide: boolean
    formatted: boolean
  }

  type Detail = Cell & { value: string | null }

  type Props = {
    columns: string[]
    rows: Row[]
    source?: string
    types?: Record<string, string>
    rowHeight?: number
    filterable?: boolean
    editable?: boolean
    locked?: boolean
    keyColumns?: string[]
    busy?: boolean
    loading?: boolean
    empty?: string
    minimap?: boolean
    wheelPan?: boolean
    spot?: Cell | null
    needle?: string
    sort?: Sort | null
    filters?: Record<string, Filter>
    pretty?: boolean
    status?: Snippet
    editCount?: (count: number) => string
    onapply?: (
      edits: CellEdit[],
    ) => Promise<boolean | undefined | void> | boolean | undefined | void
    onblocked?: (retry: () => void) => void
    onsort?: (sort: Sort | null) => void
    onfilter?: (filters: Record<string, Filter>) => void
    onmore?: () => void
    onjump?: (column: string, value: string) => void
    actions?: (row: number) => MenuItem[]
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
        | "dropFilter"
        | "contains"
        | "starts"
        | "ends"
        | "isnull"
        | "notnull"
        | "apply"
        | "discard"
        | "edited"
        | "copyAll"
        | "inspect"
        | "jumpTo"
        | "noKey"
        | "value"
        | "loading"
        | "pretty"
        | "cancel"
        | "close"
        | "sort"
        | "resize"
        | "ascending"
        | "descending"
        | "unsorted"
        | "filter"
        | "edit"
        | "filters",
        string
      >
    >
  }

  let {
    columns,
    rows,
    source = "",
    types = {},
    rowHeight = rem(2),
    filterable = true,
    editable = false,
    locked = false,
    keyColumns = [],
    busy = false,
    loading = false,
    empty = "",
    minimap = true,
    wheelPan = true,
    spot = null,
    needle = "",
    sort: sortFrom = undefined,
    filters: filtersFrom = undefined,
    pretty = $bindable(false),
    status,
    editCount,
    onapply,
    onblocked,
    onsort,
    onfilter,
    onmore,
    onjump,
    actions,
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
    dropFilter: labels.dropFilter ?? "Remove filter",
    contains: labels.contains ?? "contains",
    starts: labels.starts ?? "starts with",
    ends: labels.ends ?? "ends with",
    isnull: labels.isnull ?? "is null",
    notnull: labels.notnull ?? "is not null",
    apply: labels.apply ?? "Apply",
    discard: labels.discard ?? "Discard",
    edited: labels.edited ?? "edited",
    noKey: labels.noKey ?? "No primary key, so edits cannot be written back",
    value: labels.value ?? "Value",
    loading: labels.loading ?? "Reading rows",
    pretty: labels.pretty ?? "Pretty print",
    cancel: labels.cancel ?? "Cancel",
    close: labels.close ?? "Close",
    sort: labels.sort ?? "Sort",
    resize: labels.resize ?? "Resize",
    ascending: labels.ascending ?? "Ascending",
    descending: labels.descending ?? "Descending",
    unsorted: labels.unsorted ?? "Unsorted",
    filter: labels.filter ?? "Filter",
    edit: labels.edit ?? "Edit",
    filters: labels.filters ?? "Active filters",
  })

  const FILTER_DELAY = 250

  const PAGE_MARGIN = 24

  const NO_VALUE = ["isnull", "notnull"]

  const LONG_VALUE = 80

  const SAMPLE = 100

  const SKELETON = ["w-12", "w-32", "w-24", "w-40", "w-20", "w-28"]

  let operators = $derived([
    { value: "contains", label: words.contains },
    { value: "eq", label: "=" },
    { value: "ne", label: "≠" },
    { value: "gt", label: ">" },
    { value: "gte", label: "≥" },
    { value: "lt", label: "<" },
    { value: "lte", label: "≤" },
    { value: "starts", label: words.starts },
    { value: "ends", label: words.ends },
    { value: "isnull", label: words.isnull },
    { value: "notnull", label: words.notnull },
  ])

  let orders = $derived([
    { value: "asc", label: words.ascending, icon: "lucide:arrow-up" },
    { value: "desc", label: words.descending, icon: "lucide:arrow-down" },
    { value: "none", label: words.unsorted, icon: "lucide:minus" },
  ])

  const uid = $props.id()
  const unit = rem(1)
  const DEFAULT_WIDTH = rem(11)
  const MIN_WIDTH = rem(4.5)
  const HEADER = rem(2)
  const POPOVER = rem(18)

  let viewport = $state<HTMLDivElement | null>(null)
  let popover = $state<HTMLDivElement | null>(null)
  let widths = $state<Record<string, number>>({})
  let filters = $state<Record<number, Filter>>({})
  let sort = $state<Order | null>(null)
  let openFilter = $state<number | null>(null)
  let filterBefore: Filter | undefined
  let filterEnding = $state<Ending>("cancel")
  let cursor = $state<Cell | null>(null)
  let editing = $state<Editing | null>(null)
  let editEnding = $state<Ending>("cancel")
  let staged = $state<Record<string, Staged>>({})
  let detail = $state<Detail | null>(null)
  let detailEnding = $state<Ending>("cancel")
  let refusal = $state("")
  let applying = $state(false)

  const needsValue = (op: string) => !NO_VALUE.includes(op)

  const live = (filter: Filter) => filter.value !== "" || !needsValue(filter.op)

  const labelOf = (op: string) =>
    operators.find(entry => entry.value === op)?.label ?? op

  let active = $derived(
    Object.entries(filters)
      .map(([index, filter]) => [Number(index), filter] as const)
      .filter(([, filter]) => live(filter)),
  )

  const outline = (set: Record<number, Filter>) =>
    JSON.stringify(
      Object.entries(set)
        .filter(([, filter]) => live(filter))
        .map(([index, filter]) => [Number(index), filter.op, filter.value]),
    )

  let heardFilters: string | null = null
  let heardSort: string | null = null
  let filterTimer: ReturnType<typeof setTimeout> | undefined

  $effect.pre(() => {
    const given = filtersFrom

    if (given === undefined) {
      return
    }

    const next: Record<number, Filter> = {}

    for (const [name, filter] of Object.entries(given)) {
      const index = columns.indexOf(name)

      if (index >= 0) {
        next[index] = { op: filter.op, value: filter.value }
      }
    }

    const heard = outline(next)

    untrack(() => {
      if (heard === heardFilters) {
        return
      }

      heardFilters = heard

      if (heard !== outline(filters)) {
        filters = next
        openFilter = null
      }
    })
  })

  $effect.pre(() => {
    const given = sortFrom

    if (given === undefined) {
      return
    }

    const index = given ? columns.indexOf(given.column) : -1
    const next: Order | null =
      given && index >= 0 ? { column: index, dir: given.dir } : null
    const heard = JSON.stringify(next)

    untrack(() => {
      if (heard === heardSort) {
        return
      }

      heardSort = heard

      if (heard !== JSON.stringify(sort)) {
        sort = next
      }
    })
  })

  $effect(() => () => clearTimeout(filterTimer))

  function reportFilters(delay = FILTER_DELAY) {
    clearTimeout(filterTimer)

    if (!onfilter) {
      return
    }

    filterTimer = setTimeout(() => {
      const said = outline(filters)

      if (said === heardFilters) {
        return
      }

      heardFilters = said
      onfilter?.(
        Object.fromEntries(
          active.map(([index, filter]) => [
            columns[index],
            { op: filter.op, value: filter.value },
          ]),
        ),
      )
    }, delay)
  }

  let shape = $derived(JSON.stringify([source, columns]))

  $effect.pre(() => {
    void shape

    untrack(() => {
      staged = {}
      editing = null
      cursor = null
      detail = null
      refusal = ""

      if (!onfilter) {
        filters = {}
        openFilter = null
      }

      if (!onsort) {
        sort = null
      }
    })
  })

  let order = $derived(JSON.stringify(sort) + outline(filters))

  $effect.pre(() => {
    void order

    untrack(() => {
      cursor = null
      editing = null
    })
  })

  let dirty = $derived(Object.keys(staged).length)
  let keyIndexes = $derived(keyColumns.map(name => columns.indexOf(name)))

  let shown = $derived.by(() => {
    if (remote) {
      return rows
    }

    let result =
      active.length === 0
        ? rows
        : rows.filter(row =>
            active.every(([index, filter]) => matches(row[index], filter)),
          )

    if (sort) {
      const index = sort.column
      const flip = sort.dir === "asc" ? 1 : -1

      result = [...result].sort(
        (a, b) => compare(a[index] ?? null, b[index] ?? null) * flip,
      )
    }

    return result
  })

  let structuredData = $derived(
    rows.slice(0, 50).some(row =>
      row.some(cell => {
        const head = cell?.trimStart()[0]

        return head === "{" || head === "["
      }),
    ),
  )

  const NUMERIC = "+-.eE0123456789"

  const numberLike = (cell: string) =>
    [...cell].every(char => NUMERIC.includes(char)) &&
    Number.isFinite(Number(cell))

  let numeric = $derived.by(() => {
    const sample = rows.slice(0, SAMPLE)

    return columns.map((_, index) => {
      let seen = false

      for (const row of sample) {
        const cell = row[index]

        if (cell == null || cell === "") {
          continue
        }

        if (!numberLike(cell)) {
          return false
        }

        seen = true
      }

      return seen
    })
  })

  let linked = $derived(columns.map(name => !!onjump && !!references[name]))

  let dense = $derived(rowHeight < rem(1.75))

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

  function sortBy(index: number, dir: "asc" | "desc" | null) {
    const next: Order | null = dir ? { column: index, dir } : null

    sort = next

    if (onsort) {
      heardSort = JSON.stringify(next)
      onsort(next ? { column: columns[index], dir: next.dir } : null)
    }
  }

  function toggleSort(index: number) {
    sortBy(
      index,
      sort?.column !== index ? "asc" : sort.dir === "asc" ? "desc" : null,
    )
  }

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

  const rowKey = (row: Row) =>
    JSON.stringify(keyIndexes.map(index => row[index] ?? null))

  const stampOf = (row: Row, column: number) => `${column}:${rowKey(row)}`

  function keysOf(row: Row) {
    return Object.fromEntries(
      keyColumns.map((name, at) => [name, row[keyIndexes[at]] ?? null]),
    )
  }

  function stagedAt(row: number, column: number) {
    const held = shown[row]

    return dirty > 0 && held ? staged[stampOf(held, column)] : undefined
  }

  function cellOf(row: number, column: number) {
    const edit = stagedAt(row, column)

    return edit ? edit.value : (shown[row]?.[column] ?? null)
  }

  const flat = new Map<string, string>()

  function display(value: string) {
    const head = value.trimStart()[0]

    if (!pretty || (head !== "{" && head !== "[")) {
      return value
    }

    let done = flat.get(value)

    if (done === undefined) {
      if (flat.size > 2000) {
        flat.clear()
      }

      done = format(value, false)
      flat.set(value, done)
    }

    return done
  }

  const widthKey = (index: number) => `${index}:${columns[index]}`

  const widthOf = (index: number) => widths[widthKey(index)] ?? DEFAULT_WIDTH

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

  let span = $derived({
    width: Math.max($columnScroller.getTotalSize(), 1),
    height: Math.max($rowScroller.getTotalSize(), 1),
  })

  let roams = $derived(
    minimap &&
      !loading &&
      (span.width > look.width + 4 || span.height > look.height + 4) &&
      shown.length > 0,
  )

  let frame = $derived.by(() => {
    const width = Math.min(Math.max((look.width / span.width) * 100, 10), 100)
    const height = Math.min(
      Math.max((look.height / span.height) * 100, 30),
      100,
    )

    return {
      width,
      height,
      left: Math.min((look.left / span.width) * 100, 100 - width),
      top: Math.min((look.top / span.height) * 100, 100 - height),
    }
  })

  let fill = $derived(
    Math.max(Math.ceil((look.height - HEADER) / rowHeight), 1),
  )

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
      })
    }

    walk(event)
    drag(event, walk)
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

    if (!remote && active.length > 0) {
      return
    }

    if (last.index >= shown.length - PAGE_MARGIN) {
      untrack(() => onmore())
    }
  })

  $effect(() => {
    const count = columns.length
    const sizes = { ...widths }
    const element = viewport

    untrack(() => {
      $columnScroller.setOptions({
        count,
        estimateSize: index => sizes[widthKey(index)] ?? DEFAULT_WIDTH,
        getScrollElement: () => element,
      })
      $columnScroller.measure()
    })
  })

  function autoFit(index: number) {
    if (!viewport) {
      return
    }

    const context = document.createElement("canvas").getContext("2d")

    if (!context) {
      return
    }

    const style = getComputedStyle(viewport)

    context.font = `${style.fontSize} ${style.fontFamily}`

    let widest = context.measureText(columns[index]).width + rem(4)

    // ponytail: first 1000 rows only, full scan if wide tails matter
    for (const row of shown.slice(0, 1000)) {
      const cell = row[index] ?? null

      widest = Math.max(
        widest,
        context.measureText(cell === null ? "NULL" : display(cell)).width,
      )
    }

    widths = {
      ...widths,
      [widthKey(index)]: Math.min(
        Math.max(Math.ceil(widest) + rem(2), MIN_WIDTH),
        rem(30),
      ),
    }
  }

  function startResize(event: PointerEvent, index: number) {
    event.stopPropagation()

    const startX = event.clientX
    const startWidth = widthOf(index)

    drag(event, moved => {
      widths = {
        ...widths,
        [widthKey(index)]: Math.max(
          MIN_WIDTH,
          startWidth + moved.clientX - startX,
        ),
      }
    })
  }

  const TAB = "\t"

  function delimited() {
    const line = (cells: (string | null)[]) =>
      cells.map(cell => cell ?? "null").join(TAB)

    return [line(columns), ...shown.map(line)].join("\n")
  }

  function copy(text: string) {
    navigator.clipboard.writeText(text)
  }

  function menuFor(row: number, column: number): MenuItem[] {
    const name = columns[column]
    const raw = cellOf(row, column)

    const items: MenuItem[] = [
      {
        label: words.copyCell,
        icon: "lucide:copy",
        run: () => copy(raw ?? ""),
      },
      {
        label: words.copyRow,
        icon: "lucide:rows-3",
        run: () =>
          copy(
            columns
              .map((_, index) => cellOf(row, index) ?? "null")
              .join(TAB),
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
        run: () => inspect({ row, column }),
      },
      ...(onjump && references[name] && raw !== null
        ? [
            {
              label: `${words.jumpTo} ${references[name]}`,
              icon: "lucide:arrow-right-to-line",
              run: () => onjump(name, raw),
            },
          ]
        : []),
      ...(filterable
        ? [
            {
              label: words.filterBy,
              icon: "lucide:filter",
              run: () => {
                filters = {
                  ...filters,
                  [column]:
                    raw === null
                      ? { op: "isnull", value: "" }
                      : { op: "eq", value: raw },
                }
                reportFilters(0)
              },
            },
            ...(active.length > 0
              ? [
                  {
                    label: words.clearFilters,
                    icon: "lucide:filter-x",
                    danger: true,
                    run: clearFilters,
                  },
                ]
              : []),
          ]
        : []),
      ...(actions?.(row) ?? []),
    ]

    return items.map(item => ({
      ...item,
      run: () => {
        item.run()

        if (!detail && !editing) {
          viewport?.focus()
        }
      },
    }))
  }

  function openMenu(event: MouseEvent, row: number, column: number) {
    focusCell(row, column)
    menu.show(event, menuFor(row, column))
  }

  function menuAtCursor(cell: Cell) {
    const node = viewport?.querySelector<HTMLElement>(
      `[data-cell="${cell.row}:${cell.column}"]`,
    )
    const box = node?.getBoundingClientRect()

    if (!box) {
      return
    }

    menu.show(
      new MouseEvent("contextmenu", {
        clientX: box.left + Math.min(box.width, rem(3)),
        clientY: box.bottom,
      }),
      menuFor(cell.row, cell.column),
    )
  }

  function wheel(event: WheelEvent) {
    if (!wheelPan || !viewport || event.deltaX !== 0) {
      return
    }

    const sideways =
      event.shiftKey || viewport.scrollHeight <= viewport.clientHeight

    if (!sideways) {
      return
    }

    event.preventDefault()
    viewport.scrollLeft += event.deltaY
  }

  function headerTip(index: number) {
    const name = columns[index]
    const lines = [name]

    if (types[name]) {
      lines.push(types[name])
    }

    if (references[name]) {
      lines.push(`→ ${references[name]}`)
    }

    const filter = filters[index]

    if (filter && live(filter)) {
      lines.push(
        needsValue(filter.op)
          ? `${labelOf(filter.op)} ${filter.value}`
          : labelOf(filter.op),
      )
    }

    if (filterable) {
      lines.push(`${words.filter}: Alt+↓`)
    }

    return lines.join("\n")
  }

  let tips = $derived(columns.map((_, index) => headerTip(index)))

  function activeOn(index: number) {
    const filter = filters[index]

    return !!filter && live(filter)
  }

  function clearFilters() {
    filters = {}
    openFilter = null
    reportFilters(0)
  }

  function startOf(index: number) {
    let start = 0

    for (let at = 0; at < index; at++) {
      start += widthOf(at)
    }

    return start
  }

  let anchor = $derived.by(() => {
    if (openFilter === null) {
      return 0
    }

    const left = startOf(openFilter) - look.left

    return Math.max(0, Math.min(left, look.width - POPOVER))
  })

  function forget(index: number) {
    if (!activeOn(index)) {
      const next = { ...filters }

      delete next[index]
      filters = next
    }
  }

  function closeFilter(as: Ending = "confirm") {
    const index = openFilter

    if (index === null) {
      return
    }

    filterEnding = as

    if (as === "cancel") {
      const next = { ...filters }

      if (filterBefore) {
        next[index] = filterBefore
      } else {
        delete next[index]
      }

      filters = next
      reportFilters(0)
    } else {
      forget(index)
    }

    openFilter = null
    viewport?.focus()
  }

  function openColumn(index: number) {
    if (!filterable) {
      return
    }

    if (openFilter !== null) {
      forget(openFilter)
    }

    filterBefore = filters[index] && { ...filters[index] }

    if (!filters[index]) {
      filters = { ...filters, [index]: { op: "contains", value: "" } }
    }

    filterEnding = "cancel"
    openFilter = index
    $columnScroller.scrollToIndex(index, { align: "auto" })
  }

  function toggleColumn(index: number) {
    if (openFilter === index) {
      closeFilter()

      return
    }

    openColumn(index)
  }

  function setFilter(index: number, patch: Partial<Filter>) {
    filters = {
      ...filters,
      [index]: {
        ...(filters[index] ?? { op: "contains", value: "" }),
        ...patch,
      },
    }
    reportFilters(patch.value === undefined ? 0 : FILTER_DELAY)
  }

  function dropFilter(index: number) {
    const next = { ...filters }

    delete next[index]
    filters = next

    if (openFilter === index) {
      filterEnding = "confirm"
      openFilter = null
      viewport?.focus()
    }

    reportFilters(0)
  }

  function outside(event: PointerEvent) {
    const target = event.target

    if (openFilter === null || !(target instanceof Element)) {
      return
    }

    if (popover?.contains(target)) {
      return
    }

    if (target.closest(`[data-column="${openFilter}"]`)) {
      return
    }

    closeFilter()
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
    refusal = ""
    $rowScroller.scrollToIndex(bounded.row, { align: "auto" })
    $columnScroller.scrollToIndex(bounded.column, { align: "auto" })
  }

  function enter(event: FocusEvent) {
    const grid = event.currentTarget

    if (
      !(grid instanceof HTMLElement) ||
      event.target !== grid ||
      cursor ||
      shown.length === 0 ||
      !grid.matches(":focus-visible")
    ) {
      return
    }

    const top = $rowScroller
      .getVirtualItems()
      .find(item => item.start >= look.top)
    const left = $columnScroller
      .getVirtualItems()
      .find(item => item.start >= look.left)

    cursor = { row: top?.index ?? 0, column: left?.index ?? 0 }
  }

  function long(value: string | null) {
    if (value === null) {
      return false
    }

    const trimmed = value.trim()

    return (
      value.length > LONG_VALUE ||
      value.includes("\n") ||
      looksStructured(value) ||
      (trimmed.startsWith("<") && trimmed.endsWith(">"))
    )
  }

  function gate(cell: Cell, run: (cell: Cell) => void) {
    if (!editable) {
      return
    }

    if (keyColumns.length === 0) {
      refusal = words.noKey

      return
    }

    if (locked) {
      onblocked?.(() => {
        if (editable && !locked && keyColumns.length > 0 && shown[cell.row]) {
          cursor = cell
          run(cell)
        }
      })

      return
    }

    run(cell)
  }

  function beginEdit(cell: Cell, seed?: string) {
    const held = shown[cell.row]

    if (!held) {
      return
    }

    const current = cellOf(cell.row, cell.column)
    const wide = seed === undefined && long(current)
    const formatted = wide && pretty && looksStructured(current)

    editEnding = "cancel"
    editing = {
      ...cell,
      source: held,
      wide,
      formatted,
      draft: seed ?? (formatted ? format(current ?? "") : (current ?? "")),
    }
  }

  function stage(row: Row, column: number, value: string | null | undefined) {
    const key = stampOf(row, column)

    if (value === undefined) {
      const rest = { ...staged }

      delete rest[key]
      staged = rest

      return
    }

    staged = { ...staged, [key]: { column, keys: keysOf(row), value } }
  }

  function commitEdit(send = false) {
    const edit = editing

    if (!edit) {
      return
    }

    editEnding = "confirm"
    editing = null
    stage(
      edit.source,
      edit.column,
      settle(edit.source[edit.column] ?? null, edit.draft, edit.formatted),
    )
    viewport?.focus()

    if (send) {
      void apply()
    }
  }

  function reformat() {
    const edit = editing

    if (!edit || !looksStructured(edit.draft)) {
      return
    }

    if (!edit.formatted) {
      editing = { ...edit, draft: format(edit.draft), formatted: true }

      return
    }

    const original = edit.source[edit.column] ?? null

    editing = {
      ...edit,
      draft: settle(original, edit.draft, true) ?? original ?? "",
      formatted: false,
    }
  }

  function cancelEdit() {
    editEnding = "cancel"
    editing = null
    viewport?.focus()
  }

  function inspect(cell: Cell) {
    detailEnding = "cancel"
    detail = { ...cell, value: cellOf(cell.row, cell.column) }
  }

  function closeDetail() {
    detailEnding = "cancel"
    detail = null
    viewport?.focus()
  }

  function editFromDetail() {
    const cell = detail

    if (!cell) {
      return
    }

    detailEnding = "confirm"
    detail = null
    viewport?.focus()
    focusCell(cell.row, cell.column)
    gate(cell, beginEdit)
  }

  function clearCell(cell: Cell) {
    const held = shown[cell.row]

    if (held) {
      stage(
        held,
        cell.column,
        (held[cell.column] ?? null) === null ? undefined : null,
      )
    }
  }

  function discard() {
    staged = {}
    editing = null
  }

  async function apply() {
    if (!onapply || dirty === 0 || applying) {
      return
    }

    const sent = Object.keys(staged)
    const byRow = new Map<string, CellEdit>()

    for (const [stamp, edit] of Object.entries(staged)) {
      const row = stamp.slice(stamp.indexOf(":") + 1)
      const entry = byRow.get(row) ?? { keys: { ...edit.keys }, set: {} }

      entry.set[columns[edit.column]] = edit.value
      byRow.set(row, entry)
    }

    applying = true

    const edits = [...byRow.values()]
    const send = onapply

    const done = await Promise.resolve()
      .then(() => send(edits))
      .catch(() => false)
      .finally(() => (applying = false))

    if (done === false) {
      return
    }

    const rest = { ...staged }

    for (const key of sent) {
      delete rest[key]
    }

    staged = rest
  }

  function grab(node: HTMLInputElement | HTMLTextAreaElement) {
    node.focus()
    node.setSelectionRange(node.value.length, node.value.length)
  }

  function cellAt(event: Event): Cell | null {
    const target = event.target as HTMLElement | null
    const hit = target?.closest<HTMLElement>("[data-cell]")?.dataset.cell

    if (!hit) {
      return null
    }

    const [row, column] = hit.split(":").map(Number)

    return { row, column }
  }

  function press(event: MouseEvent) {
    const cell = cellAt(event)

    if (cell) {
      focusCell(cell.row, cell.column)
    }
  }

  function doublePress(event: MouseEvent) {
    const cell = cellAt(event)

    if (cell) {
      focusCell(cell.row, cell.column)
      gate(cell, beginEdit)
    }
  }

  function contextPress(event: MouseEvent) {
    const cell = cellAt(event)

    if (cell) {
      openMenu(event, cell.row, cell.column)
    } else {
      event.preventDefault()
    }
  }

  function inlineKeys(event: KeyboardEvent) {
    if (event.isComposing) {
      return
    }

    if (event.key === "Enter") {
      event.preventDefault()
      commitEdit(true)

      return
    }

    if (event.key === "Escape") {
      event.preventDefault()
      event.stopPropagation()
      cancelEdit()

      return
    }

    if (event.key === "Tab" && editing) {
      const from = editing

      event.preventDefault()
      commitEdit()
      focusCell(from.row, from.column + (event.shiftKey ? -1 : 1))
    }
  }

  function keys(event: KeyboardEvent) {
    if (event.target !== event.currentTarget || editing || event.isComposing) {
      return
    }

    if (!cursor) {
      if (event.key.startsWith("Arrow") && shown.length > 0) {
        event.preventDefault()
        focusCell(0, 0)
      }

      return
    }

    const here = cursor
    const writable = editable && !locked && keyColumns.length > 0

    const moves: Record<string, [number, number]> = {
      ArrowDown: [1, 0],
      ArrowUp: [-1, 0],
      ArrowRight: [0, 1],
      ArrowLeft: [0, -1],
      Enter: [1, 0],
      PageDown: [fill, 0],
      PageUp: [-fill, 0],
    }

    if (event.key === "ArrowDown" && event.altKey) {
      event.preventDefault()
      openColumn(here.column)

      return
    }

    const menuKey =
      event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)

    if (menuKey) {
      event.preventDefault()
      menuAtCursor(here)

      return
    }

    if (
      (event.key === "Enter" && writable && !event.ctrlKey) ||
      event.key === "F2"
    ) {
      event.preventDefault()
      gate(here, beginEdit)

      return
    }

    if (event.key === "Home") {
      event.preventDefault()
      focusCell(event.ctrlKey ? 0 : here.row, 0)

      return
    }

    if (event.key === "End") {
      event.preventDefault()
      focusCell(
        event.ctrlKey ? shown.length - 1 : here.row,
        columns.length - 1,
      )

      return
    }

    if (event.key === "Delete" || event.key === "Backspace") {
      if (editable) {
        event.preventDefault()
        gate(here, clearCell)
      }

      return
    }

    if (event.key === "c" && event.ctrlKey) {
      event.preventDefault()
      copy(cellOf(here.row, here.column) ?? "null")

      return
    }

    const step = moves[event.key]

    if (step) {
      event.preventDefault()
      focusCell(here.row + step[0], here.column + step[1])

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
      beginEdit(here, event.key)
    }
  }

  let readout = $derived(
    cursor && shown[cursor.row] && columns[cursor.column] !== undefined
      ? cursor
      : null,
  )
</script>

<svelte:window onpointerdown={outside} />

<div in:fade={veil()} class="flex min-h-0 min-w-0 flex-1 flex-col">
  <div class="relative flex min-h-0 min-w-0 flex-1 flex-col">
    <div
      bind:this={viewport}
      role="grid"
      tabindex="0"
      aria-rowcount={shown.length + 1}
      aria-colcount={columns.length}
      aria-busy={loading || busy}
      aria-activedescendant={cursor
        ? `${uid}-${cursor.row}-${cursor.column}`
        : undefined}
      onkeydown={keys}
      onfocus={enter}
      onwheel={wheel}
      onscroll={watch}
      onclick={press}
      ondblclick={doublePress}
      oncontextmenu={contextPress}
      class={[
        "group/grid relative min-h-0 flex-1 overflow-auto outline-none",
        "select-none",
        dense ? "text-xs" : "text-sm",
      ]}
      style:scrollbar-gutter="stable"
    >
      {#if loading}
        <div
          aria-hidden="true"
          class="sticky left-0 overflow-hidden"
          style:width="{look.width / unit}rem"
        >
          <div
            class={[
              "flex h-8 items-center gap-6 border-b border-base-content/10",
              "px-3",
            ]}
          >
            {#each SKELETON as size, index (index)}
              <span class={["skeleton h-2 shrink-0", size]}></span>
            {/each}
          </div>

          {#each { length: fill }, line (line)}
            <div
              class={[
                "flex items-center gap-6 border-b border-base-content/5 px-3",
              ]}
              style:height="{rowHeight / unit}rem"
            >
              {#each SKELETON as _, index (index)}
                <span
                  class={[
                    "skeleton h-2 shrink-0 opacity-60",
                    SKELETON[(index + line) % SKELETON.length],
                  ]}
                ></span>
              {/each}
            </div>
          {/each}
        </div>
      {:else}
        <div
          role="rowgroup"
          class="sticky top-0 z-20 bg-base-100"
          style:width="{$columnScroller.getTotalSize() / unit}rem"
        >
          <div
            role="row"
            aria-rowindex={1}
            class="relative h-8 border-b border-base-content/15"
          >
            {#each $columnScroller.getVirtualItems() as column (column.key)}
              {@const index = column.index}
              {@const name = columns[index]}
              {@const sorted = sort?.column === index ? sort.dir : null}
              {@const filtered = activeOn(index)}
              {@const aimed = cursor?.column === index}

              <div
                role="columnheader"
                aria-colindex={index + 1}
                aria-sort={sorted === "asc"
                  ? "ascending"
                  : sorted === "desc"
                    ? "descending"
                    : "none"}
                data-column={index}
                class={[
                  "absolute top-0 flex h-8 items-center",
                  "border-r border-base-content/5",
                  (filtered || openFilter === index) && "bg-primary/10",
                ]}
                style:left="{column.start / unit}rem"
                style:width="{column.size / unit}rem"
              >
                <button
                  type="button"
                  tabindex="-1"
                  onclick={() => toggleColumn(index)}
                  use:tooltip={tips[index]}
                  class={[
                    "flex h-full min-w-0 flex-1 cursor-pointer items-center",
                    "gap-2 pl-3 text-left text-xs transition-colors",
                    aimed || filtered ? "text-primary" : "text-base-content",
                  ]}
                >
                  <span class="truncate font-semibold">{name}</span>

                  {#if types[name]}
                    <span class="truncate text-base-content/70 lowercase">
                      {types[name]}
                    </span>
                  {/if}

                  {#if keyColumns.includes(name)}
                    <Icon
                      icon="lucide:key-round"
                      class="size-3 shrink-0 text-accent"
                    />
                  {/if}

                  {#if linked[index]}
                    <Icon
                      icon="lucide:arrow-up-right"
                      class="size-3 shrink-0 text-info"
                    />
                  {/if}

                  {#if filtered}
                    <Icon icon="lucide:filter" class="size-3 shrink-0" />
                  {/if}
                </button>

                <button
                  type="button"
                  tabindex="-1"
                  aria-label="{words.sort} {name}"
                  onclick={() => {
                    toggleSort(index)
                    viewport?.focus()
                  }}
                  class={[
                    "grid size-6 shrink-0 cursor-pointer place-items-center",
                    "transition-colors",
                    sorted
                      ? "text-primary"
                      : "text-base-content/60 hover:text-base-content",
                  ]}
                >
                  <Icon
                    icon={sorted === "asc"
                      ? "lucide:arrow-up"
                      : sorted === "desc"
                        ? "lucide:arrow-down"
                        : "lucide:arrow-up-down"}
                    class="size-3"
                  />
                </button>

                <button
                  type="button"
                  tabindex="-1"
                  aria-label="{words.resize} {name}"
                  onpointerdown={event => startResize(event, index)}
                  ondblclick={event => {
                    event.stopPropagation()
                    autoFit(index)
                  }}
                  class={[
                    "h-8 w-1 shrink-0 cursor-col-resize bg-transparent",
                    "transition-colors hover:bg-primary/60",
                  ]}
                ></button>
              </div>
            {/each}
          </div>
        </div>

        <div
          role="rowgroup"
          in:fade|local={veil()}
          class="relative"
          style:height="{$rowScroller.getTotalSize() / unit}rem"
          style:width="{$columnScroller.getTotalSize() / unit}rem"
        >
          {#each $rowScroller.getVirtualItems() as row (row.key)}
            {@const current = cursor?.row === row.index}

            <div
              role="row"
              aria-rowindex={row.index + 2}
              class={[
                "absolute inset-x-0 border-b border-base-content/5",
                "contain-paint",
                current ? "bg-primary/5" : "hover:bg-base-content/5",
              ]}
              style:height="{row.size / unit}rem"
              style:transform="translateY({row.start / unit}rem)"
            >
              {#each $columnScroller.getVirtualItems() as column (column.key)}
                {@const edit = stagedAt(row.index, column.index)}
                {@const cell = edit
                  ? edit.value
                  : (shown[row.index]?.[column.index] ?? null)}
                {@const here = current && cursor?.column === column.index}
                {@const found =
                  spot != null &&
                  shown[row.index] === rows[spot.row] &&
                  spot.column === column.index}
                {@const match =
                  needle !== "" &&
                  cell != null &&
                  cell.toLowerCase().includes(needle)}
                {@const inline =
                  !!editing &&
                  !editing.wide &&
                  editing.row === row.index &&
                  editing.column === column.index}

                {#if editing && inline}
                  <input
                    use:grab
                    value={editing.draft}
                    aria-label={columns[column.index]}
                    oninput={event => {
                      if (editing) {
                        editing.draft = event.currentTarget.value
                      }
                    }}
                    onblur={() => commitEdit()}
                    onkeydown={inlineKeys}
                    class={[
                      "absolute z-10 h-full bg-base-100 px-3 outline-none",
                      "select-text ring-2 ring-primary ring-inset",
                      numeric[column.index] && "text-right",
                    ]}
                    style:left="{column.start / unit}rem"
                    style:width="{column.size / unit}rem"
                  />
                {:else}
                  <span
                    id="{uid}-{row.index}-{column.index}"
                    role="gridcell"
                    aria-colindex={column.index + 1}
                    aria-selected={here}
                    data-cell="{row.index}:{column.index}"
                    class={[
                      "absolute h-full truncate border-r border-base-content/5",
                      "px-3",
                      numeric[column.index] && "text-right",
                      cell !== null &&
                        linked[column.index] && [
                          "underline decoration-base-content/40",
                          "decoration-dotted underline-offset-4",
                        ],
                      found
                        ? "bg-accent/30 ring-2 ring-accent ring-inset"
                        : match
                          ? "bg-accent/15"
                          : edit
                            ? "bg-primary/10 font-medium text-primary"
                            : here && "bg-primary/10",
                      here &&
                        !found && [
                          "ring-2 ring-base-content/40 ring-inset",
                          "group-focus-within/grid:ring-primary",
                        ],
                    ]}
                    style:left="{column.start / unit}rem"
                    style:width="{column.size / unit}rem"
                    style:line-height="{row.size / unit}rem"
                    title={cell ?? "NULL"}
                  >
                    {#if cell === null}
                      <span class="text-xs text-base-content/70">NULL</span>
                    {:else}
                      {display(cell)}
                    {/if}
                  </span>
                {/if}
              {/each}
            </div>
          {/each}
        </div>

        {#if shown.length === 0 && empty !== "" && !busy}
          <div
            in:fade|local={veil()}
            class="sticky left-0 grid place-items-center"
            style:width="{look.width / unit}rem"
            style:height="{Math.max(look.height - HEADER, 0) / unit}rem"
          >
            <EmptyState art="sheet" title={empty}>
              {#if active.length > 0}
                <button
                  type="button"
                  onclick={clearFilters}
                  class="btn btn-soft btn-sm"
                >
                  <Icon icon="lucide:filter-x" class="size-4" />
                  {words.clearFilters}
                </button>
              {/if}
            </EmptyState>
          </div>
        {/if}
      {/if}
    </div>

    {#if openFilter !== null}
      {@const index = openFilter}
      {@const current = filters[index] ?? { op: "contains", value: "" }}

      <div
        bind:this={popover}
        in:rise
        out:leave={{ as: filterEnding }}
        role="dialog"
        aria-label="{words.filter} {columns[index]}"
        tabindex="-1"
        onkeydown={event => {
          if (event.key === "Escape" && !event.defaultPrevented) {
            event.preventDefault()
            event.stopPropagation()
            closeFilter("cancel")
          }
        }}
        class={[
          "hud hud-lit floating lift absolute top-9 z-30 flex w-72 flex-col",
          "gap-3 p-3 text-xs outline-none",
        ]}
        style:left="{anchor / unit}rem"
      >
        <p class="flex min-w-0 items-center gap-2">
          <span class="truncate text-sm font-semibold">{columns[index]}</span>

          {#if types[columns[index]]}
            <span class="truncate text-base-content/70 lowercase">
              {types[columns[index]]}
            </span>
          {/if}
        </p>

        <Segmented
          small
          label={words.sort}
          options={orders}
          value={sort?.column === index ? sort.dir : "none"}
          onpick={next =>
            sortBy(index, next === "asc" || next === "desc" ? next : null)}
        />

        <div class="flex flex-col gap-1">
          <span class="text-base-content/70">{words.filter}</span>

          <div class="flex gap-2">
            <div class={needsValue(current.op) ? "w-28 shrink-0" : "flex-1"}>
              <Dropdown
                wide
                small
                label={words.filter}
                value={current.op}
                options={operators}
                onpick={op => setFilter(index, { op })}
              />
            </div>

            {#if needsValue(current.op)}
              <input
                use:grab
                value={current.value}
                aria-label={words.value}
                oninput={event =>
                  setFilter(index, { value: event.currentTarget.value })}
                onkeydown={event => {
                  if (event.key === "Enter" && !event.isComposing) {
                    event.preventDefault()
                    closeFilter()
                  }
                }}
                placeholder={words.value}
                class={[
                  "input input-sm min-w-0 flex-1 bg-base-100 select-text",
                  "placeholder:text-base-content/60",
                ]}
              />
            {/if}
          </div>
        </div>

        <div class="flex items-center gap-2">
          <span class="flex-1"></span>

          {#if activeOn(index)}
            <button
              type="button"
              onclick={() => dropFilter(index)}
              class="btn btn-ghost btn-sm"
            >
              {words.dropFilter}
            </button>
          {/if}

          <button
            type="button"
            onclick={() => closeFilter()}
            class="btn btn-primary btn-sm font-medium"
          >
            {words.apply}
          </button>
        </div>
      </div>
    {/if}
  </div>

  <footer
    class={[
      "flex h-10 shrink-0 items-center gap-3 border-t border-base-content/10",
      "px-3 text-xs text-base-content/70 tabular-nums",
    ]}
  >
    <span class="grid size-4 shrink-0 place-items-center" aria-live="polite">
      {#if busy || paging || loading}
        <span transition:fade|local={veil()} class="grid place-items-center">
          <Icon
            icon="lucide:loader-circle"
            class="size-4 animate-spin text-primary"
          />
          <span class="sr-only">{words.loading}</span>
        </span>
      {/if}
    </span>

    <div class="flex min-w-0 flex-1 items-center gap-3 overflow-hidden">
      {#if status}
        {@render status()}
      {/if}

      {#if active.length > 0}
        <ul
          aria-label={words.filters}
          class="flex min-w-0 items-center gap-1 overflow-x-auto"
        >
          {#each active as [index, filter] (index)}
            <li
              animate:flip={{ duration: TIMING.quick }}
              transition:scale|local={pop()}
              class="flex shrink-0 items-center bg-primary/10 text-primary"
            >
              <button
                type="button"
                onclick={() => openColumn(index)}
                class="flex cursor-pointer items-center gap-1 py-1 pl-2"
              >
                <span class="font-medium">{columns[index]}</span>
                <span>{labelOf(filter.op)}</span>

                {#if needsValue(filter.op)}
                  <span class="max-w-24 truncate">{filter.value}</span>
                {/if}
              </button>

              <button
                type="button"
                aria-label="{words.dropFilter} {columns[index]}"
                onclick={() => dropFilter(index)}
                class={[
                  "grid size-6 cursor-pointer place-items-center",
                  "transition-colors hover:text-error",
                ]}
              >
                <Icon icon="lucide:x" class="size-3" />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    {#if dirty > 0}
      <div
        in:fade|local={veil()}
        role="status"
        class="flex shrink-0 items-center gap-2"
      >
        <Icon icon="lucide:pencil" class="size-4 text-primary" />

        <span class="text-base-content">
          {editCount ? editCount(dirty) : `${dirty} ${words.edited}`}
        </span>

        <button type="button" onclick={discard} class="btn btn-ghost btn-sm">
          {words.discard}
        </button>

        <button
          type="button"
          onclick={apply}
          disabled={busy || applying}
          class="btn btn-primary btn-sm font-medium"
        >
          {words.apply}
        </button>
      </div>
    {:else if refusal}
      <p
        in:fade|local={veil()}
        role="status"
        class="flex min-w-0 shrink items-center gap-2"
      >
        <Icon icon="lucide:key-round" class="size-4 shrink-0 text-warning" />
        <span class="truncate">{refusal}</span>
      </p>
    {:else if readout}
      <p class="flex min-w-0 shrink items-center gap-2">
        <span class="shrink-0">
          {(readout.row + 1).toLocaleString()} / {shown.length.toLocaleString()}
        </span>

        <span class="max-w-40 truncate font-medium text-base-content">
          {columns[readout.column]}
        </span>

        {#if types[columns[readout.column]]}
          <span class="max-w-24 truncate lowercase">
            {types[columns[readout.column]]}
          </span>
        {/if}
      </p>
    {/if}

    {#if structuredData && !loading}
      <button
        type="button"
        aria-pressed={pretty}
        onclick={() => (pretty = !pretty)}
        class={[
          "btn btn-ghost btn-sm shrink-0 gap-2 font-normal",
          pretty && "text-primary",
        ]}
      >
        <Icon icon="lucide:braces" class="size-4" />
        {words.pretty}
      </button>
    {/if}

    {#if roams}
      <button
        type="button"
        tabindex="-1"
        aria-hidden="true"
        transition:fade|local={veil()}
        onpointerdown={roam}
        class={[
          "relative h-6 w-24 shrink-0 cursor-crosshair overflow-hidden",
          "bg-base-content/5 hairline",
        ]}
      >
        <span
          class="absolute border border-primary bg-primary/20"
          style:left="{frame.left}%"
          style:top="{frame.top}%"
          style:width="{frame.width}%"
          style:height="{frame.height}%"
        ></span>
      </button>
    {/if}
  </footer>
</div>

{#if editing?.wide}
  {@const edit = editing}

  <Dialog
    label={columns[edit.column]}
    onclose={cancelEdit}
    dismiss={words.cancel}
    size="lg"
    ending={editEnding}
  >
    <header class="flex min-w-0 items-center gap-2 px-6 pt-6 pb-4">
      <h2 class="truncate text-base font-semibold tracking-tight">
        {columns[edit.column]}
      </h2>

      {#if types[columns[edit.column]]}
        <span class="truncate text-xs text-base-content/70 lowercase">
          {types[columns[edit.column]]}
        </span>
      {/if}

      <span class="flex-1"></span>

      {#if looksStructured(edit.source[edit.column] ?? null)}
        <button
          type="button"
          aria-pressed={edit.formatted}
          disabled={!looksStructured(edit.draft)}
          onclick={reformat}
          class={[
            "btn btn-ghost btn-sm gap-2 font-normal",
            edit.formatted && "text-primary",
          ]}
        >
          <Icon icon="lucide:braces" class="size-4" />
          {words.pretty}
        </button>
      {/if}
    </header>

    <div class="min-h-0 flex-1 px-6">
      <textarea
        use:grab
        value={edit.draft}
        aria-label={columns[edit.column]}
        spellcheck="false"
        oninput={event => {
          if (editing) {
            editing.draft = event.currentTarget.value
          }
        }}
        onkeydown={event => {
          if (event.isComposing) {
            return
          }

          if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
            event.preventDefault()
            commitEdit(true)
          }
        }}
        class={[
          "textarea h-80 max-h-full w-full resize-none bg-base-100 text-sm",
          "leading-6 select-text",
        ]}
      ></textarea>
    </div>

    <footer class="flex items-center gap-2 px-6 pt-4 pb-6">
      <Keycap keys={["ctrl", "enter"]} class="flex-1" />

      <button type="button" onclick={cancelEdit} class="btn btn-ghost btn-sm">
        {words.cancel}
      </button>

      <button
        type="button"
        onclick={() => commitEdit(true)}
        class="btn btn-primary btn-sm font-medium"
      >
        {words.apply}
      </button>
    </footer>
  </Dialog>
{/if}

{#if detail}
  {@const held = detail}
  {@const name = columns[held.column] ?? ""}
  {@const structured = looksStructured(held.value)}
  {@const text =
    held.value === null
      ? "NULL"
      : pretty && structured
        ? format(held.value)
        : held.value}

  <Dialog
    label={name}
    onclose={closeDetail}
    dismiss={words.close}
    size="lg"
    ending={detailEnding}
  >
    <header class="flex min-w-0 items-center gap-2 px-6 pt-6 pb-4">
      <h2 class="truncate text-base font-semibold tracking-tight">{name}</h2>

      {#if types[name]}
        <span class="truncate text-xs text-base-content/70 lowercase">
          {types[name]}
        </span>
      {/if}

      <span class="flex-1"></span>

      <button
        type="button"
        aria-label={words.close}
        onclick={closeDetail}
        class="btn btn-square btn-ghost btn-sm"
      >
        <Icon icon="lucide:x" class="size-4" />
      </button>
    </header>

    <div class="min-h-0 flex-1 overflow-auto px-6">
      <pre
        class={[
          "bg-base-200 p-4 text-sm leading-6 whitespace-pre-wrap wrap-anywhere",
          "select-text hairline",
          held.value === null && "text-base-content/70",
        ]}>{text}</pre>
    </div>

    <footer class="flex items-center gap-2 px-6 pt-4 pb-6">
      {#if structured}
        <button
          type="button"
          aria-pressed={pretty}
          onclick={() => (pretty = !pretty)}
          class={["btn btn-ghost btn-sm gap-2", pretty && "text-primary"]}
        >
          <Icon icon="lucide:braces" class="size-4" />
          {words.pretty}
        </button>
      {/if}

      <span class="flex-1"></span>

      <button
        type="button"
        onclick={() => copy(held.value ?? "")}
        class="btn btn-soft btn-sm"
      >
        <Icon icon="lucide:copy" class="size-4" />
        {words.copyCell}
      </button>

      {#if editable}
        <button
          type="button"
          onclick={editFromDetail}
          class="btn btn-primary btn-sm font-medium"
        >
          <Icon icon="lucide:pencil" class="size-4" />
          {words.edit}
        </button>
      {/if}
    </footer>
  </Dialog>
{/if}
