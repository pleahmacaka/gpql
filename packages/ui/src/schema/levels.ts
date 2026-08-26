import type { Edge, Node } from "@xyflow/svelte"

import type { TableGroup } from "./board.svelte"
import type { SchemaTable } from "../types"

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

const BAND_PAD = 16
const BAND_HEAD = 34

function settle(tables: SchemaTable[], groups: TableGroup[]) {
  const home = new Map<string, string>()

  for (const group of groups) {
    for (const name of group.tables) {
      home.set(name, group.id)
    }
  }

  const leans = new Map<string, Set<string>>(
    groups.map(group => [group.id, new Set<string>()]),
  )

  for (const table of tables) {
    const mine = home.get(table.name)

    if (!mine) {
      continue
    }

    for (const parent of parentsOf(table)) {
      const theirs = home.get(parent)

      if (theirs && theirs !== mine) {
        leans.get(mine)?.add(theirs)
      }
    }
  }

  const depth = new Map<string, number>()

  const dig = (id: string, walking: Set<string>): number => {
    const known = depth.get(id)

    if (known !== undefined) {
      return known
    }

    if (walking.has(id)) {
      return 0
    }

    walking.add(id)

    const behind = [...(leans.get(id) ?? [])]
    const value = behind.length
      ? Math.max(...behind.map(other => dig(other, walking) + 1))
      : 0

    walking.delete(id)
    depth.set(id, value)

    return value
  }

  return [...groups].sort(
    (left, right) => dig(left.id, new Set()) - dig(right.id, new Set()),
  )
}

function bands(
  tables: SchemaTable[],
  groups: TableGroup[],
): { nodes: Node[]; loose: SchemaTable[]; width: number } {
  const found = new Map(tables.map(table => [table.name, table]))
  const taken = new Set<string>()
  const nodes: Node[] = []
  let x = 0

  for (const group of settle(tables, groups)) {
    const members = byLevel(
      group.tables
        .map(name => found.get(name))
        .filter((table): table is SchemaTable => table !== undefined),
    ).flatMap(column => column.tables)

    if (members.length === 0) {
      continue
    }

    let y = BAND_HEAD

    const inside: Node[] = members.map(table => {
      const node: Node = {
        id: table.name,
        type: "table",
        position: { x: BAND_PAD, y },
        data: { table },
        parentId: group.id,
        extent: "parent" as const,
        deletable: false,
      }

      taken.add(table.name)
      y += cardHeight(table) + ROW_GAP

      return node
    })

    const width = NODE_WIDTH + BAND_PAD * 2
    const height = y - ROW_GAP + BAND_PAD * 2

    nodes.push({
      id: group.id,
      type: "band",
      position: { x, y: 0 },
      data: { id: group.id, name: group.name, count: members.length },
      width,
      height,
      style: `width: ${width}px; height: ${height}px`,
      selectable: true,
      deletable: false,
    })

    nodes.push(...inside)
    x += NODE_WIDTH + BAND_PAD * 2 + COLUMN_GAP
  }

  return {
    nodes,
    loose: tables.filter(table => !taken.has(table.name)),
    width: x,
  }
}

export function toFlow(
  tables: SchemaTable[],
  groups: TableGroup[] = [],
  rest = "rest",
) {
  const known = new Set(tables.map(table => table.name))
  const nodes: Node[] = []
  const edges: Edge[] = []

  const held = groups.length > 0 ? bands(tables, groups) : null
  const free = held ? held.loose : tables
  const shift = held ? held.width : 0

  if (held) {
    nodes.push(...held.nodes)
  }

  const columns = byLevel(free)

  columns.forEach((column, index) => {
    const x = shift + index * (NODE_WIDTH + COLUMN_GAP)
    let y = 0

    nodes.push({
      id: `level:${index}`,
      type: "level",
      position: { x, y: -34 },
      data: { label: held ? `${rest}, ${column.label}` : column.label },
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
