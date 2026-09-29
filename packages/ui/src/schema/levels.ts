import type { Edge, Node } from "@xyflow/svelte"

import { rem } from "../controls/rem"
import type { SchemaTable } from "../types"
import type { TableGroup } from "./board.svelte"

export type Column = { label: string; depth: number; tables: SchemaTable[] }

export const CARD = {
  width: 18,
  header: 2.5,
  row: 1.5,
  line: 1,
  inset: 0.25,
}

const NOTE_WIDTH = 40
const NOTE_LINES = 2
const COLUMN_GAP = 4
const ROW_GAP = 2
const BAND_PAD = 1
const BAND_HEAD = 2.5
const LEVEL_LIFT = 2
const SWEEPS = 4

export function relationCount(tables: SchemaTable[]) {
  return tables.reduce(
    (total, table) =>
      total + table.columns.filter(column => column.references).length,
    0,
  )
}

export function splitReference(reference: string) {
  const cut = reference.lastIndexOf(".")

  return cut === -1
    ? { table: reference, column: "" }
    : { table: reference.slice(0, cut), column: reference.slice(cut + 1) }
}

function targetOf(reference: string) {
  return splitReference(reference).table
}

function parentsOf(table: SchemaTable) {
  return table.columns
    .filter(column => column.references)
    .map(column => targetOf(column.references ?? ""))
    .filter(name => name !== table.name)
}

function distinctTables(tables: SchemaTable[]) {
  const seen = new Set<string>()

  return tables.filter(table => {
    if (seen.has(table.name)) {
      return false
    }

    seen.add(table.name)

    return true
  })
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
      depth: key,
      tables: grouped.get(key) ?? [],
    }))

  return tidy(columns)
}

function tidy(columns: Column[]) {
  const place = new Map<string, number>()

  for (const column of columns) {
    for (const [index, table] of column.tables.entries()) {
      place.set(table.name, index)
    }
  }

  for (let sweep = 0; sweep < SWEEPS; sweep += 1) {
    for (const column of columns.slice(1)) {
      column.tables.sort((left, right) => pull(left) - pull(right))
      for (const [index, table] of column.tables.entries()) {
        place.set(table.name, index)
      }
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

const wide = (char: string) => (char.codePointAt(0) ?? 0) >= 0x2e80

function weight(line: string) {
  let total = 0

  for (const char of line) {
    total += wide(char) ? 2 : 1
  }

  return total
}

export function noteRows(table: SchemaTable) {
  const note = table.note?.trim() ?? ""

  if (note === "") {
    return 0
  }

  const rows = note
    .split(/\r?\n/)
    .reduce(
      (total, line) =>
        total + Math.max(1, Math.ceil(weight(line) / NOTE_WIDTH)),
      0,
    )

  return Math.min(NOTE_LINES, rows)
}

export function noteHeight(table: SchemaTable) {
  const rows = noteRows(table)

  return rows === 0 ? 0 : rows * CARD.line + 1
}

function head(table: SchemaTable) {
  return (
    CARD.header + noteHeight(table) + (table.policies?.length ?? 0) * CARD.row
  )
}

export function columnOffset(table: SchemaTable, index: number) {
  return rem(head(table) + CARD.inset + index * CARD.row + CARD.row / 2)
}

export function cardHeight(table: SchemaTable) {
  return rem(head(table) + CARD.inset * 2 + table.columns.length * CARD.row)
}

export const nodeCentre = () => rem(CARD.width / 2)

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
  const width = rem(CARD.width + BAND_PAD * 2)
  let x = 0

  for (const group of settle(tables, groups)) {
    const members = byLevel(
      group.tables
        .filter(
          (name, spot, all) => !taken.has(name) && all.indexOf(name) === spot,
        )
        .map(name => found.get(name))
        .filter((table): table is SchemaTable => table !== undefined),
    ).flatMap(column => column.tables)

    if (members.length === 0) {
      continue
    }

    const band = `band:${group.id}`
    let y = rem(BAND_HEAD)

    const inside: Node[] = members.map(table => {
      const node: Node = {
        id: table.name,
        type: "table",
        position: { x: rem(BAND_PAD), y },
        data: { table },
        parentId: band,
        extent: "parent" as const,
        deletable: false,
      }

      taken.add(table.name)
      y += cardHeight(table) + rem(ROW_GAP)

      return node
    })

    const height = y - rem(ROW_GAP) + rem(BAND_PAD)

    nodes.push({
      id: band,
      type: "band",
      position: { x, y: 0 },
      data: { id: group.id, name: group.name, count: members.length },
      width,
      height,
      style: `width: ${width / rem(1)}rem; height: ${height / rem(1)}rem`,
      selectable: true,
      deletable: false,
    })

    nodes.push(...inside)
    x += width + rem(COLUMN_GAP)
  }

  return {
    nodes,
    loose: tables.filter(table => !taken.has(table.name)),
    width: x,
  }
}

const link = (id: string, source: string, handle: string, target: string) => ({
  id,
  source,
  sourceHandle: handle,
  target,
  targetHandle: "referenced",
  type: "step",
  selectable: false,
  deletable: false,
})

export function toFlow(
  listed: SchemaTable[],
  groups: TableGroup[] = [],
  rest = "rest",
) {
  const tables = distinctTables(listed)
  const known = new Set(tables.map(table => table.name))
  const nodes: Node[] = []
  const edges: Edge[] = []

  const held = groups.length > 0 ? bands(tables, groups) : null
  const free = held ? held.loose : tables
  const shift = held ? held.width : 0

  if (held) {
    nodes.push(...held.nodes)
  }

  for (const [index, column] of byLevel(free).entries()) {
    const x = shift + index * rem(CARD.width + COLUMN_GAP)
    let y = 0

    nodes.push({
      id: `level:${index}`,
      type: "level",
      position: { x, y: -rem(LEVEL_LIFT) },
      data: {
        label: held ? `${rest}, ${column.label}` : column.label,
        depth: column.depth,
        grouped: held !== null,
      },
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

      y += cardHeight(table) + rem(ROW_GAP)
    }
  }

  for (const table of tables) {
    for (const [index, column] of table.columns.entries()) {
      const parent = column.references ? targetOf(column.references) : ""

      if (parent && known.has(parent)) {
        edges.push(
          link(`${table.name}:${index}`, table.name, `column:${index}`, parent),
        )
      }
    }

    for (const [index, hint] of (table.hints ?? []).entries()) {
      const parent = targetOf(hint)

      if (known.has(parent) && parent !== table.name) {
        edges.push({
          ...link(`${table.name}~${index}`, table.name, "note", parent),
          class: "edge-hint",
        })
      }
    }
  }

  return { nodes, edges }
}
