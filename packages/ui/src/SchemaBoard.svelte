<script lang="ts">
  import {
    Background,
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

  import { board } from "./board.svelte"
  import { Icon } from "./icons"
  import LevelNode from "./LevelNode.svelte"
  import { byLevel, toFlow } from "./levels"
  import SchemaTableNode from "./SchemaTableNode.svelte"
  import type { SchemaTable } from "./types"

  type Props = {
    tables: SchemaTable[]
    dark?: boolean
    keyboard?: boolean
    room?: string
    tidyLabel?: string
    onselect?: (table: string) => void
  }

  let {
    tables,
    dark = false,
    keyboard = true,
    room = "",
    tidyLabel = "Tidy",
    onselect,
  }: Props = $props()

  let shared: Y.Map<{ x: number; y: number }> | null = null

  const nodeTypes = { table: SchemaTableNode, level: LevelNode }
  const flow = useSvelteFlow()

  let nodes = $state.raw<Node[]>([])
  let edges = $state.raw<Edge[]>([])
  let linked = $state.raw<Edge[]>([])

  let levels = $derived(byLevel(tables))

  $effect(() => {
    const built = toFlow(tables)

    nodes = built.nodes
    linked = built.edges
  })

  $effect(() => {
    const selected = board.selected

    edges = linked.map(edge => {
      const touches = edge.source === selected || edge.target === selected

      return {
        ...edge,
        animated: touches,
        style: touches
          ? "stroke: var(--color-primary); stroke-width: 1.5"
          : "stroke: color-mix(in oklch, currentColor 20%, transparent)",
      }
    })
  })

  $effect(() => {
    const target = board.selected
    const ready = nodes.length > 0

    if (!target || !ready) {
      return
    }

    untrack(() => {
      board.table = target
      board.column = -1
      flow.fitView({ nodes: [{ id: target }], duration: 220, maxZoom: 1 })
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
    if (!shared) {
      return
    }

    for (const node of nodes) {
      shared.set(node.id, { x: node.position.x, y: node.position.y })
    }
  }

  function tidy() {
    const built = toFlow(tables)

    nodes = built.nodes
    linked = built.edges
    flow.fitView({ padding: 0.12, maxZoom: 1, duration: 240 })
    publish()
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
    flow.fitView({ nodes: [{ id: table }], duration: 180, maxZoom: 1 })
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
  bind:nodes
  bind:edges
  {nodeTypes}
  colorMode={dark ? "dark" : "light"}
  fitView
  fitViewOptions={{ padding: 0.12, maxZoom: 1 }}
  minZoom={0.15}
  nodesConnectable={false}
  elementsSelectable={false}
  onnodedragstop={publish}
  onnodeclick={({ node }) => {
    board.table = node.id
    board.column = -1
    onselect?.(node.id)
  }}
>
  <Background gap={24} />
  <Controls showLock={false} />

  <Panel position="top-right">
    <button
      type="button"
      onclick={tidy}
      class="flex items-center gap-1.5 rounded-field bg-base-100 px-2 py-1
        text-xs hairline hover:bg-base-300"
    >
      <Icon icon="lucide:wand-sparkles" class="size-3.5" />
      {tidyLabel}
    </button>
  </Panel>
</SvelteFlow>
