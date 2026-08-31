<script lang="ts">
  import {
    Background,
    MiniMap,
    Controls,
    Panel,
    SvelteFlow,
    useSvelteFlow,
    type Edge,
    type Node,
  } from "@xyflow/svelte"
  import "@xyflow/svelte/dist/style.css"

  import { untrack } from "svelte"
  import * as Y from "yjs"

  import { menu, type MenuItem } from "../controls/menu.svelte"
  import { board, type TableGroup } from "./board.svelte"
  import { Icon } from "../icons"
  import BandNode from "./BandNode.svelte"
  import LevelNode from "./LevelNode.svelte"
  import { NODE_CENTRE, byLevel, columnOffset, toFlow } from "./levels"
  import SchemaTableNode from "./SchemaTableNode.svelte"
  import type { SchemaTable } from "../types"

  type Props = {
    tables: SchemaTable[]
    dark?: boolean
    keyboard?: boolean
    minimap?: boolean
    room?: string
    labels?: Partial<
      Record<
        | "auto"
        | "picked"
        | "group"
        | "ungroup"
        | "warn"
        | "groupName"
        | "think"
        | "nothing"
        | "rest"
        | "define"
        | "cancel",
        string
      >
    >
    onselect?: (table: string) => void
    onsuggest?: (signal: AbortSignal) => Promise<TableGroup[]> | TableGroup[]
    onlayout?: (layout: {
      spots: Record<string, { x: number; y: number }>
      groups: TableGroup[]
    }) => void
  }

  let {
    tables,
    dark = false,
    keyboard = true,
    room = "",
    labels = {},
    minimap = true,
    onselect,
    onsuggest,
    onlayout,
  }: Props = $props()

  let words = $derived({
    auto: labels.auto ?? "Auto arrange",
    picked: labels.picked ?? "Arrange picked",
    group: labels.group ?? "Group",
    ungroup: labels.ungroup ?? "Ungroup",
    warn: labels.warn ?? "positions reset, groups stay. click again",
    groupName: labels.groupName ?? "group",
    think: labels.think ?? "Group with AI",
    nothing: labels.nothing ?? "the model found no grouping worth keeping",
    rest: labels.rest ?? "everything else",
    define: labels.define ?? "Show definition",
    cancel: labels.cancel ?? "Cancel",
  })

  let armed = $state(false)

  let shared: Y.Map<{ x: number; y: number }> | null = null

  const nodeTypes = { table: SchemaTableNode, level: LevelNode, band: BandNode }
  const flow = useSvelteFlow()

  let nodes = $state.raw<Node[]>([])
  let edges = $state.raw<Edge[]>([])
  let linked = $state.raw<Edge[]>([])

  let plain = $derived($state.snapshot(tables) as SchemaTable[])

  let levels = $derived(byLevel(plain))

  $effect(() => {
    const built = toFlow(plain, board.groups, words.rest)
    // reading the saved spots without tracking them keeps dragging from
    // feeding back into a rebuild
    const saved = untrack(() => board.spots)

    nodes = built.nodes.map(node =>
      saved[node.id] ? { ...node, position: saved[node.id] } : node,
    )
    linked = built.edges
  })

  // the table in hand keeps its relations bright, the rest step back
  $effect(() => {
    const focus = board.table ?? board.selected
    const near = new Set(board.groupOf(focus ?? "")?.tables ?? [])
    const drawn = linked.length > 0 ? linked : untrack(() => edges)

    edges = drawn.map(edge => {
      const mine =
        !focus ||
        edge.source === focus ||
        edge.target === focus ||
        (near.has(edge.source) && near.has(edge.target))

      return {
        ...edge,
        style: mine
          ? "stroke: color-mix(in oklch, var(--color-primary) 70%, transparent); stroke-width: 1.75"
          : "stroke: color-mix(in oklch, currentColor 12%, transparent); stroke-width: 1",
      }
    })
  })

  $effect(() => {
    const target = board.selected

    if (!target) {
      return
    }

    untrack(() => {
      if (nodes.length === 0) {
        return
      }

      board.table = target
      board.column = -1

      const table = plain.find(entry => entry.name === target)
      const index =
        board.needle && table
          ? table.columns.findIndex(column =>
              column.name.toLowerCase().includes(board.needle),
            )
          : -1

      if (index >= 0 && table) {
        board.column = index

        const spot =
          flow.getInternalNode(target)?.internals.positionAbsolute ??
          nodes.find(node => node.id === target)?.position

        if (spot) {
          flow.setCenter(
            spot.x + NODE_CENTRE,
            spot.y + columnOffset(table, index),
            { zoom: 1, duration: 220 },
          )

          return
        }
      }

      flow.fitView({
        nodes: [{ id: target }],
        duration: 220,
        maxZoom: 1,
        minZoom: 1,
      })
    })
  })

  $effect(() => {
    if (room === "") {
      return
    }

    const doc = new Y.Doc()
    const positions = doc.getMap<{ x: number; y: number }>("positions")
    const socket = new WebSocket(room)

    socket.binaryType = "arraybuffer"
    shared = positions

    const post = (bytes: Uint8Array) => {
      socket.send(bytes.slice().buffer)
    }

    socket.onopen = () => post(Y.encodeStateAsUpdate(doc))
    socket.onmessage = event =>
      Y.applyUpdate(doc, new Uint8Array(event.data), "remote")

    doc.on("update", (update: Uint8Array, origin: unknown) => {
      if (origin !== "remote" && socket.readyState === WebSocket.OPEN) {
        post(update)
      }
    })

    positions.observe(() => {
      nodes = nodes.map(node => {
        const moved = positions.get(node.id)

        return moved ? { ...node, position: moved } : node
      })
    })

    return () => {
      shared = null
      socket.close()
      doc.destroy()
    }
  })

  function publish() {
    const spots: Record<string, { x: number; y: number }> = {}

    for (const node of nodes) {
      if (node.type !== "table") {
        continue
      }

      spots[node.id] = { x: node.position.x, y: node.position.y }
      shared?.set(node.id, spots[node.id])
    }

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
        id: `g${kept.length + 1}`,
        name: `${words.groupName} ${kept.length + 1}`,
        tables: [...board.picked],
      },
    ]

    regroup()
  }

  let thinking = $state<AbortController | null>(null)
  let notice = $state("")

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

      board.groups = found
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

    nodes = built.nodes
    linked = built.edges
    board.spots = {}
    flow.fitView({ padding: 0.12, maxZoom: 1, duration: 240 })
    publish()
  }

  $effect(() => {
    board.rename = rename
    board.ungroup = dropGroup

    return () => {
      board.rename = null
      board.ungroup = null
    }
  })

  function rename(id: string, name: string) {
    board.groups = board.groups.map(group =>
      group.id === id ? { ...group, name } : group,
    )
    publish()
  }

  function openMenu(event: MouseEvent, table: string) {
    event.preventDefault()

    if (table !== "" && !board.picked.includes(table)) {
      board.picked = [table]
    }

    const held = table === "" ? undefined : board.groupOf(table)
    const items: MenuItem[] = []

    if (held === undefined && board.ondefine) {
      items.push({
        label: words.define,
        icon: "lucide:file-code-2",
        run: () => board.ondefine?.(table),
      })
    }

    if (board.picked.length > 1) {
      items.push({
        label: words.group,
        icon: "lucide:group",
        run: makeGroup,
      })
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
      items.push({
        label: words.think,
        icon: "lucide:sparkles",
        run: suggest,
      })
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

  function dropGroup(id: string) {
    board.groups = board.groups.filter(group => group.id !== id)
    regroup()
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

    nodes = nodes.map(node => {
      if (!picked.has(node.id)) {
        return node
      }

      const placed = {
        ...node,
        position: { x: anchor.x, y: anchor.y + offset },
      }

      offset += 28 + 54 + 24 + 22 * (tables.find(t => t.name === node.id)?.columns.length ?? 3)

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
      duration: 180,
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

  function keys(event: KeyboardEvent) {
    if (!keyboard) {
      return
    }

    const target = event.target as HTMLElement | null

    if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) {
      return
    }

    if (event.key === "Escape" && (armed || thinking)) {
      event.preventDefault()
      armed = false
      thinking?.abort()

      return
    }

    const steps: Record<string, () => void> = {
      ArrowDown: () => move(1, "row"),
      ArrowUp: () => move(-1, "row"),
      ArrowRight: () => move(1, "level"),
      ArrowLeft: () => move(-1, "level"),
      Enter: () => {
        if (board.table) {
          onselect?.(board.table)
        }
      },
    }

    const step = steps[event.key]

    if (step) {
      event.preventDefault()
      step()
    }
  }
</script>

<svelte:window onkeydown={keys} />

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
  panOnDrag={[1, 2]}
  panOnScroll
  onnodedragstop={publish}
  onnodecontextmenu={({ event, node }) =>
    openMenu(event as MouseEvent, node.id)}
  onselectioncontextmenu={({ event }) => openMenu(event as MouseEvent, "")}
  onpaneclick={() => {
    board.picked = []
    board.table = null
    board.column = -1
  }}
  onpanecontextmenu={({ event }) => openMenu(event as MouseEvent, "")}
  onselectionchange={collect}
  onnodeclick={({ node }) => {
    board.table = node.id
    board.column = -1
    onselect?.(node.id)
  }}
>
  <Background gap={24} />

  {#if minimap}
    <MiniMap
      position="bottom-right"
      width={128}
      height={80}
      nodeStrokeWidth={2}
      nodeBorderRadius={2}
      class="rounded-field floating lift"
    />
  {/if}
  <Controls showLock={false} />

  <Panel position="top-right">
    <div class="flex flex-col items-end gap-1">
      <div class="flex items-center gap-1">
        {#if board.picked.length > 1}
          <button
            type="button"
            onclick={makeGroup}
            class="flex items-center gap-2 rounded-field bg-base-100 px-2 py-1
              text-xs hairline hover:bg-base-300"
          >
            <Icon icon="lucide:group" class="size-4" />
            {words.group}
          </button>
        {/if}

        {#if onsuggest}
          <button
            type="button"
            onclick={suggest}
            class="flex items-center gap-2 rounded-field bg-base-100 px-2 py-1
              text-xs hairline hover:bg-base-300"
          >
            <Icon
              icon={thinking ? "lucide:loader-circle" : "lucide:sparkles"}
              class="size-4 {thinking ? 'animate-spin' : 'text-accent'}"
            />
            {thinking ? words.cancel : words.think}
          </button>
        {/if}

        <button
          type="button"
          onclick={board.picked.length > 0 ? arrangePicked : arrangeAll}
          class="flex items-center gap-2 rounded-field px-2 py-1 text-xs
            hairline {armed
            ? 'bg-primary text-primary-content'
            : 'bg-base-100 hover:bg-base-300'}"
        >
          <Icon icon="lucide:wand-sparkles" class="size-4" />
          {board.picked.length > 0 ? words.picked : words.auto}
        </button>
      </div>

      {#if notice}
        <p
          class="max-w-72 rounded-field floating px-2 py-1 text-right text-xs
            break-keep text-error lift"
        >
          {notice}
        </p>
      {/if}

      {#if armed && board.picked.length === 0}
        <button
          type="button"
          onclick={() => (armed = false)}
          class="max-w-72 rounded-field floating px-2 py-1 text-right text-xs
            break-keep text-base-content/60 lift hover:text-base-content"
        >
          {words.warn} · {words.cancel}
        </button>
      {/if}

      {#if board.groups.length > 0}
        <div class="flex max-w-64 flex-wrap justify-end gap-1">
          {#each board.groups as group (group.id)}
            <span
              class="flex items-center gap-1 rounded-selector bg-primary/10 pr-1
                pl-2 text-xs text-primary"
            >
              <span class="py-1">{group.name}</span>

              <span class="opacity-60">{group.tables.length}</span>

              <button
                type="button"
                aria-label="{words.ungroup} {group.name}"
                onclick={() => dropGroup(group.id)}
                class="rounded-selector p-1 opacity-60 hover:opacity-100"
              >
                <Icon icon="lucide:x" class="size-3" />
              </button>
            </span>
          {/each}
        </div>
      {/if}
    </div>
  </Panel>
</SvelteFlow>

