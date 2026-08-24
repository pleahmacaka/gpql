import type { Edge, Node } from "@xyflow/svelte"

import type { SchemaTable } from "./types"

export type Column = { label: string; tables: SchemaTable[] }

const NODE_WIDTH = 288
const COLUMN_GAP = 64
const ROW_GAP = 28
const HEADER_HEIGHT = 54
const LINE_HEIGHT = 22
const CARD_PADDING = 24
const SWEEPS = 4

export function relationCount(tables: SchemaTable[]) {
  return tables.reduce(
    (total, table) =>
      total + table.columns.filter(column => column.references).length,
    0,
  )
}

function parentsOf(table: SchemaTable) {
  return table.columns
    .filter(column => column.references)
    .map(column => column.references?.split(".")[0] ?? "")
    .filter(name => name !== table.name)
}

export function byLevel(tables: SchemaTable[]): Column[] {
  const targets = new Map(tables.map(table => [table.name, parentsOf(table)]))
  const level = new Map<string, number>()

  const depth = (name: string, walking: Set<string>): number => {
    const known = level.get(name)

    if (known !== undefined) {
      return known
    }

    if (walking.has(name)) {
      return 0
    }

    walking.add(name)

    const parents = (targets.get(name) ?? []).filter(parent =>
      targets.has(parent),
    )
    const value = parents.length
      ? Math.max(...parents.map(parent => depth(parent, walking) + 1))
      : 0

    walking.delete(name)
    level.set(name, value)

    return value
  }

  const grouped = new Map<number, SchemaTable[]>()

  for (const table of tables) {
    const key = depth(table.name, new Set())
    const bucket = grouped.get(key) ?? []

    bucket.push(table)
    grouped.set(key, bucket)
  }

  const columns = [...grouped.keys()]
    .sort((a, b) => a - b)
    .map(key => ({
      label: key === 0 ? "referenced" : `level ${key}`,
      tables: grouped.get(key) ?? [],
    }))

  return tidy(columns)
}

function tidy(columns: Column[]) {
  const place = new Map<string, number>()

  for (const column of columns) {
    column.tables.forEach((table, index) => place.set(table.name, index))
  }

  for (let sweep = 0; sweep < SWEEPS; sweep += 1) {
    for (const column of columns.slice(1)) {
      column.tables.sort((left, right) => pull(left) - pull(right))
      column.tables.forEach((table, index) => place.set(table.name, index))
    }
  }

  return columns

  function pull(table: SchemaTable) {
    const anchors = parentsOf(table)
      .map(parent => place.get(parent))
      .filter((row): row is number => row !== undefined)

    if (anchors.length === 0) {
      return place.get(table.name) ?? 0
    }

    return anchors.reduce((total, row) => total + row, 0) / anchors.length
  }
}

function cardHeight(table: SchemaTable) {
  const notes = table.note ? LINE_HEIGHT : 0

  return HEADER_HEIGHT + table.columns.length * LINE_HEIGHT + CARD_PADDING + notes
}

export function toFlow(tables: SchemaTable[]) {
  const columns = byLevel(tables)
  const known = new Set(tables.map(table => table.name))

  const nodes: Node[] = []
  const edges: Edge[] = []

  columns.forEach((column, index) => {
    const x = index * (NODE_WIDTH + COLUMN_GAP)
    let y = 0

    nodes.push({
      id: `level:${index}`,
      type: "level",
      position: { x, y: -34 },
      data: { label: column.label },
      draggable: false,
      selectable: false,
      deletable: false,
    })

    for (const table of column.tables) {
      nodes.push({
        id: table.name,
        type: "table",
        position: { x, y },
        data: { table },
        deletable: false,
      })

      y += cardHeight(table) + ROW_GAP
    }
  })

  for (const table of tables) {
    for (const column of table.columns) {
      const parent = column.references?.split(".")[0]

      if (!parent || !known.has(parent)) {
        continue
      }

      edges.push({
        id: `${table.name}.${column.name}`,
        source: table.name,
        sourceHandle: column.name,
        target: parent,
        targetHandle: "referenced",
        type: "smoothstep",
        selectable: false,
        deletable: false,
      })
    }

    for (const hint of table.hints ?? []) {
      const parent = hint.split(".")[0]

      if (!known.has(parent) || parent === table.name) {
        continue
      }

      edges.push({
        id: `${table.name}~${hint}`,
        source: table.name,
        sourceHandle: "note",
        target: parent,
        targetHandle: "referenced",
        type: "smoothstep",
        selectable: false,
        deletable: false,
        animated: false,
        style: "stroke-dasharray: 4 4",
      })
    }
  }

  return { nodes, edges }
}
