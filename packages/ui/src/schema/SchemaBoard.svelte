<script lang="ts">
  import {
    Background,
    BackgroundVariant,
    Controls,
    type Edge,
    MiniMap,
    type Node,
    Panel,
    SvelteFlow,
    useSvelteFlow,
  } from "@xyflow/svelte"
  import "@xyflow/svelte/dist/style.css"

  import { untrack } from "svelte"

  import { type MenuItem, menu } from "../controls/menu.svelte"
  import { rem } from "../controls/rem"
  import { Icon } from "../icons"
  import { calm, TIMING } from "../motion"
  import type { LayoutChannel, SchemaTable } from "../types"
  import BandNode from "./BandNode.svelte"
  import BoardTools from "./BoardTools.svelte"
  import {
    board,
    distinct,
    type Spot,
    speak,
    type TableGroup,
    WORDS,
    type Words,
  } from "./board.svelte"
  import { shareLayout } from "./layout"
  import LevelNode from "./LevelNode.svelte"
  import {
    byLevel,
    cardHeight,
    columnOffset,
    nodeCentre,
    toFlow,
  } from "./levels"
  import SchemaTableNode from "./SchemaTableNode.svelte"

  type Props = {
    tables: SchemaTable[]
    dark?: boolean
    keyboard?: boolean
    minimap?: boolean
    scrollPan?: boolean
    channel?: LayoutChannel
    labels?: Partial<Words>
    menuItems?: (table: string | null) => MenuItem[]
    onselect?: (table: string) => void
    onsuggest?: (signal: AbortSignal) => Promise<TableGroup[]> | TableGroup[]
    onlayout?: (layout: {
      spots: Record<string, Spot>
      groups: TableGroup[]
    }) => void
  }

  let {
    tables,
    dark = false,
    keyboard = true,
    minimap = true,
    scrollPan = true,
    channel,
    labels = {},
    menuItems,
    onselect,
    onsuggest,
    onlayout,
  }: Props = $props()

  let words = $derived<Words>({ ...WORDS, ...labels })

  speak(() => words)

  const hint = $props.id()
  const nodeTypes = { table: SchemaTableNode, level: LevelNode, band: BandNode }
  const flow = useSvelteFlow()

  let nodes = $state.raw<Node[]>([])
  let edges = $state.raw<Edge[]>([])
  let linked = $state.raw<Edge[]>([])

  let armed = $state(false)
  let gliding = $state(false)
  let thinking = $state<AbortController | null>(null)
  let notice = $state("")

  let frame = $state<HTMLDivElement | null>(null)
  let centred: string | null = null
  let shared: ReturnType<typeof shareLayout> | null = null
  let settle: ReturnType<typeof setTimeout> | undefined

  let plain = $derived($state.snapshot(tables) as SchemaTable[])

  let levels = $derived(byLevel(plain))

  let spoken = $derived.by(() => {
    const table = plain.find(entry => entry.name === board.table)
    const column = table?.columns[board.column]

    if (!table || !column) {
      return table?.name ?? ""
    }

    const traits = [
      column.dataType,
      column.primaryKey && words.primary,
      !column.required && !column.primaryKey && words.nullable,
      column.references && `${words.references} ${column.references}`,
    ]

    return [`${table.name}.${column.name}`, ...traits]
      .filter(Boolean)
      .join(", ")
  })

  $effect(() => {
    const built = toFlow(plain, board.groups, words.rest)
    const saved = untrack(() => board.spots)

    nodes = built.nodes.map(node =>
      saved[node.id] ? { ...node, position: saved[node.id] } : node,
    )
    linked = built.edges

    const target = untrack(() => (keyboard ? board.selected : null))

    if (target) {
      requestAnimationFrame(() => requestAnimationFrame(() => reveal(target)))
    }
  })

  $effect(() => {
    const focus = board.hover ?? board.table ?? board.selected
    const near = new Set(board.groupOf(focus ?? "")?.tables ?? [])

    edges = linked.map(edge => {
      const mine =
        edge.source === focus ||
        edge.target === focus ||
        (near.has(edge.source) && near.has(edge.target))
      const tone = focus === null ? "" : mine ? "edge-near" : "edge-far"
      const hint = edge.sourceHandle === "note" ? "edge-hint" : ""

      return { ...edge, class: `${hint} ${tone}`.trim() }
    })
  })

  $effect(() => {
    const target = board.selected
    const ready = keyboard && nodes.length > 0

    if (!target || !ready || target === centred) {
      return
    }

    untrack(() => centre(target))
  })

  $effect(() => {
    const line = channel

    if (!line) {
      return
    }

    const layout = untrack(() =>
      shareLayout(line, (spots, whole) => {
        board.spots = whole ? spots : { ...board.spots, ...spots }
        nodes = nodes.map(node =>
          spots[node.id] ? { ...node, position: spots[node.id] } : node,
        )
      }),
    )

    shared = layout

    return () => {
      layout.stop()
      shared = null
    }
  })

  $effect(() => {
    board.rename = rename
    board.ungroup = dropGroup

    return () => {
      board.rename = null
      board.ungroup = null
      clearTimeout(settle)
    }
  })

  const GRID = "color-mix(in oklch, var(--color-base-content) 6%, transparent)"

  const pace = () => (calm() ? 0 : TIMING.base)

  function centre(target: string) {
    centred = target
    board.table = target
    board.column = -1

    const table = plain.find(entry => entry.name === target)
    const index =
      board.needle && table
        ? table.columns.findIndex(column =>
            column.name.toLowerCase().includes(board.needle),
          )
        : -1

    const spot =
      flow.getInternalNode(target)?.internals.positionAbsolute ??
      nodes.find(node => node.id === target)?.position

    if (index >= 0 && table && spot) {
      board.column = index
      flow.setCenter(
        spot.x + nodeCentre(),
        spot.y + columnOffset(table, index),
        { zoom: 1, duration: pace() },
      )

      return
    }

    flow.fitView({
      nodes: [{ id: target }],
      duration: pace(),
      maxZoom: 1,
      minZoom: 1,
    })
  }

  function reveal(target: string) {
    const node = flow.getInternalNode(target)
    const box = frame?.getBoundingClientRect()

    if (!node || !box) {
      return
    }

    const { x, y, zoom } = flow.getViewport()
    const spot = node.internals.positionAbsolute
    const width = node.measured.width ?? 0
    const height = node.measured.height ?? 0
    const left = spot.x * zoom + x
    const top = spot.y * zoom + y

    const inside =
      left >= 0 &&
      top >= 0 &&
      left + width * zoom <= box.width &&
      top + height * zoom <= box.height

    if (!inside) {
      flow.setCenter(spot.x + width / 2, spot.y + height / 2, {
        zoom,
        duration: pace(),
      })
    }
  }

  function glide() {
    if (calm()) {
      return
    }

    clearTimeout(settle)
    gliding = true
    settle = setTimeout(() => (gliding = false), TIMING.slow)
  }

  function publish() {
    const spots: Record<string, Spot> = {}

    for (const node of nodes) {
      if (node.type !== "level") {
        spots[node.id] = { x: node.position.x, y: node.position.y }
      }
    }

    board.spots = spots
    shared?.keep(spots)
    onlayout?.({ spots, groups: $state.snapshot(board.groups) })
  }

  function collect() {
    board.picked = nodes
      .filter(node => node.selected && node.type === "table")
      .map(node => node.id)
  }

  function makeGroup() {
    if (board.picked.length < 2) {
      return
    }

    const taken = new Set(board.picked)
    const kept = board.groups
      .map(group => ({
        ...group,
        tables: group.tables.filter(name => !taken.has(name)),
      }))
      .filter(group => group.tables.length > 1)

    board.groups = [
      ...kept,
      {
        id: crypto.randomUUID(),
        name: `${words.groupName} ${kept.length + 1}`,
        tables: [...board.picked],
      },
    ]

    regroup()
  }

  async function suggest() {
    if (thinking) {
      thinking.abort()

      return
    }

    if (!onsuggest) {
      return
    }

    const stopper = new AbortController()

    thinking = stopper
    notice = ""

    try {
      const found = await onsuggest(stopper.signal)

      if (found.length === 0) {
        notice = words.nothing

        return
      }

      board.groups = distinct(found)
      regroup()
    } catch (failure) {
      if (!stopper.signal.aborted) {
        notice = String(failure).replace(/^Error:\s*/, "")
      }
    } finally {
      if (thinking === stopper) {
        thinking = null
      }
    }
  }

  function regroup() {
    const built = toFlow(plain, board.groups, words.rest)

    glide()
    nodes = built.nodes
    linked = built.edges
    board.spots = {}
    flow.fitView({ padding: 0.12, maxZoom: 1, duration: pace() })
    publish()
  }

  function rename(id: string, name: string) {
    board.groups = board.groups.map(group =>
      group.id === id ? { ...group, name } : group,
    )
    publish()
  }

  function dropGroup(id: string) {
    board.groups = board.groups.filter(group => group.id !== id)
    regroup()
  }

  function showGroup(id: string) {
    flow.fitView({
      nodes: [{ id: `band:${id}` }],
      padding: 0.12,
      maxZoom: 1,
      duration: pace(),
    })
  }

  function openMenu(event: MouseEvent, table: string) {
    event.preventDefault()

    if (table !== "" && !board.picked.includes(table)) {
      board.picked = [table]
    }

    const held = table === "" ? undefined : board.groupOf(table)
    const items = [...(menuItems?.(table === "" ? null : table) ?? [])]

    if (table !== "" && board.ondefine) {
      items.push({
        label: words.define,
        icon: "lucide:file-code-2",
        run: () => board.ondefine?.(table),
      })
    }

    if (board.picked.length > 1) {
      items.push({ label: words.group, icon: "lucide:group", run: makeGroup })
    }

    if (held) {
      items.push({
        label: words.ungroup,
        icon: "lucide:ungroup",
        run: () => dropGroup(held.id),
      })
    }

    if (board.picked.length > 0) {
      items.push({
        label: words.picked,
        icon: "lucide:wand-sparkles",
        run: arrangePicked,
      })
    }

    if (onsuggest) {
      items.push({ label: words.think, icon: "lucide:sparkles", run: suggest })
    }

    items.push({
      label: words.auto,
      icon: "lucide:wand-sparkles",
      run: () => {
        armed = true
        arrangeAll()
      },
    })

    menu.show(event, items)
  }

  function arrangePicked() {
    const picked = new Set(board.picked)

    if (picked.size === 0) {
      return
    }

    const anchor = nodes
      .filter(node => picked.has(node.id))
      .reduce(
        (corner, node) => ({
          x: Math.min(corner.x, node.position.x),
          y: Math.min(corner.y, node.position.y),
        }),
        { x: Number.POSITIVE_INFINITY, y: Number.POSITIVE_INFINITY },
      )

    let offset = 0

    glide()
    nodes = nodes.map(node => {
      const table = plain.find(entry => entry.name === node.id)

      if (!picked.has(node.id) || !table) {
        return node
      }

      const placed = {
        ...node,
        position: { x: anchor.x, y: anchor.y + offset },
      }

      offset += cardHeight(table) + rem(2)

      return placed
    })

    publish()
  }

  function arrangeAll() {
    if (!armed) {
      armed = true

      return
    }

    armed = false
    regroup()
  }

  function place(table: string) {
    for (const [level, group] of levels.entries()) {
      const row = group.tables.findIndex(entry => entry.name === table)

      if (row !== -1) {
        return { level, row }
      }
    }

    return null
  }

  function land(table: string | undefined, column: number) {
    if (!table) {
      return
    }

    board.table = table
    board.column = column
    flow.fitView({
      nodes: [{ id: table }],
      duration: calm() ? 0 : TIMING.quick,
      maxZoom: 1,
      minZoom: 1,
    })
  }

  function move(step: number, axis: "row" | "level") {
    if (levels.length === 0) {
      return
    }

    const current = board.table ? place(board.table) : null

    if (!current) {
      land(levels[0].tables[0]?.name, -1)

      return
    }

    const group = levels[current.level]

    if (axis === "level") {
      const target = levels[current.level + step]
      const row = Math.min(current.row, (target?.tables.length ?? 1) - 1)

      land(target?.tables[row]?.name, -1)

      return
    }

    const table = group.tables[current.row]
    const next = board.column + step

    if (next >= -1 && next < table.columns.length) {
      board.column = next

      return
    }

    const sibling = group.tables[current.row + step]

    if (sibling) {
      land(sibling.name, step > 0 ? -1 : sibling.columns.length - 1)
    }
  }

  function enter() {
    const open = board.onopen ?? onselect

    if (board.table) {
      open?.(board.table)
    }
  }

  function keys(event: KeyboardEvent) {
    if (!keyboard || event.defaultPrevented || event.isComposing) {
      return
    }

    if (event.key === "Escape" && (armed || thinking)) {
      event.preventDefault()
      armed = false
      thinking?.abort()

      return
    }

    const target = event.target

    if (
      target instanceof HTMLElement &&
      target.closest(
        "input, textarea, select, [contenteditable], [role='dialog'], " +
          "button:not([data-cursor])",
      )
    ) {
      return
    }

    const steps: Record<string, () => void> = {
      ArrowDown: () => move(1, "row"),
      ArrowUp: () => move(-1, "row"),
      ArrowRight: () => move(1, "level"),
      ArrowLeft: () => move(-1, "level"),
      Enter: enter,
    }

    const step = steps[event.key]

    if (step) {
      event.preventDefault()
      step()
    }
  }
</script>

<svelte:window onkeydown={keys} />

<div
  bind:this={frame}
  class={["board relative size-full", gliding && "gliding"]}
>
  <button
    type="button"
    data-cursor
    aria-describedby={hint}
    onclick={enter}
    class={[
      "absolute top-4 left-4 z-10 flex items-center gap-2 floating px-3 py-2",
      "text-xs hairline",
      "pointer-events-none opacity-0 focus-visible:pointer-events-auto",
      "focus-visible:opacity-100",
    ]}
  >
    <Icon icon="lucide:keyboard" class="size-4 shrink-0 text-primary" />
    {spoken || words.board}
  </button>

  <SvelteFlow
    proOptions={{ hideAttribution: true }}
    bind:nodes
    bind:edges
    {nodeTypes}
    colorMode={dark ? "dark" : "light"}
    fitView
    fitViewOptions={{ padding: 0.12, maxZoom: 1 }}
    minZoom={0.15}
    nodesConnectable={false}
    elementsSelectable
    selectionOnDrag
    disableKeyboardA11y
    panOnDrag={[1, 2]}
    panOnScroll={scrollPan}
    zoomOnScroll={scrollPan}
    preventScrolling={scrollPan}
    ariaLabelConfig={{
      "controls.ariaLabel": words.controls,
      "controls.zoomIn.ariaLabel": words.zoomIn,
      "controls.zoomOut.ariaLabel": words.zoomOut,
      "controls.fitView.ariaLabel": words.fit,
      "minimap.ariaLabel": words.minimap,
      "node.a11yDescription.default": words.keys,
      "node.a11yDescription.keyboardDisabled": words.keys,
    }}
    onnodedragstop={publish}
    onnodecontextmenu={({ event, node }) =>
      openMenu(event as MouseEvent, node.type === "table" ? node.id : "")}
    onselectioncontextmenu={({ event }) => openMenu(event as MouseEvent, "")}
    onpaneclick={() => {
      board.picked = []
      board.table = null
      board.column = -1
    }}
    onpanecontextmenu={({ event }) => openMenu(event as MouseEvent, "")}
    onselectionchange={collect}
    onnodepointerenter={({ node }) =>
      (board.hover = node.type === "table" ? node.id : null)}
    onnodepointerleave={() => (board.hover = null)}
    onnodeclick={({ node }) => {
      if (node.type !== "table") {
        return
      }

      centred = node.id
      board.table = node.id
      board.column = -1
      onselect?.(node.id)
    }}
  >
    <Background
      variant={BackgroundVariant.Lines}
      gap={rem(4)}
      bgColor="var(--color-base-200)"
      patternColor={GRID}
    />

    {#if minimap}
      <MiniMap
        position="bottom-right"
        width={rem(8)}
        height={rem(5)}
        pannable
        class="hairline"
      />
    {/if}

    <Controls showLock={false} />

    <Panel position="top-right">
      <BoardTools
        {words}
        picked={board.picked.length}
        groups={board.groups}
        {armed}
        thinking={thinking !== null}
        {notice}
        ongroup={makeGroup}
        onsuggest={onsuggest ? suggest : undefined}
        onarrange={board.picked.length > 0 ? arrangePicked : arrangeAll}
        ondisarm={() => (armed = false)}
        ondismiss={() => (notice = "")}
        onshow={showGroup}
        ondrop={dropGroup}
      />
    </Panel>
  </SvelteFlow>

  <p id={hint} class="sr-only">{words.keys}</p>
  <p class="sr-only" aria-live="polite">{spoken}</p>
</div>

<style>
  .board {
    --xy-edge-stroke: color-mix(
      in oklch,
      var(--color-base-content) 24%,
      transparent
    );
    --xy-controls-button-background-color: var(--color-base-100);
    --xy-controls-button-background-color-hover: var(--color-base-300);
    --xy-controls-button-color: var(--color-base-content);
    --xy-controls-button-color-hover: var(--color-base-content);
    --xy-controls-button-border-color: transparent;
    --xy-controls-box-shadow: 0 0 0 0.0625rem
      color-mix(in oklch, var(--color-base-content) 10%, transparent);
    --xy-minimap-background-color: var(--color-base-100);
    --xy-minimap-mask-background-color: color-mix(
      in oklch,
      var(--color-base-300) 60%,
      transparent
    );
    --xy-minimap-mask-stroke-color: var(--color-primary);
    --xy-minimap-node-background-color: color-mix(
      in oklch,
      var(--color-base-content) 22%,
      transparent
    );
  }

  .board :global(.svelte-flow__edge-path) {
    transition:
      stroke 140ms ease-out,
      stroke-width 140ms ease-out;
  }

  .board :global(.edge-hint .svelte-flow__edge-path) {
    stroke-dasharray: 0.25rem 0.25rem;
  }

  .board :global(.svelte-flow__edges) {
    transition: opacity 140ms ease-out;
  }

  .board.gliding :global(.svelte-flow__edges) {
    opacity: 0;
    transition: none;
  }

  .board.gliding :global(.svelte-flow__node) {
    transition: transform 260ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }
</style>
