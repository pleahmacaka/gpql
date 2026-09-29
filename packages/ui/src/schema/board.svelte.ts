import { getContext, setContext } from "svelte"

export type TableGroup = {
  id: string
  name: string
  tables: string[]
}

export type Spot = { x: number; y: number }

export const WORDS = {
  auto: "Auto arrange",
  picked: "Arrange picked",
  group: "Group",
  ungroup: "Ungroup",
  rename: "Rename group",
  warn: "positions reset, groups stay. click again",
  groupName: "group",
  think: "Group with AI",
  nothing: "the model found no grouping worth keeping",
  rest: "everything else",
  define: "Show definition",
  cancel: "Cancel",
  dismiss: "Dismiss",
  open: "Open rows",
  referenced: "Referenced",
  level: "Level",
  primary: "primary key",
  nullable: "nullable",
  references: "references",
  board: "Schema diagram",
  keys: "Arrow keys move between tables and columns, Enter opens the table",
  controls: "Zoom controls",
  zoomIn: "Zoom in",
  zoomOut: "Zoom out",
  fit: "Fit to view",
  minimap: "Minimap",
}

export type Words = typeof WORDS

const SPOKEN = Symbol("schema words")

export const speak = (words: () => Words) => setContext(SPOKEN, words)

export const spoken = () => getContext<() => Words>(SPOKEN) ?? (() => WORDS)

export function distinct(groups: TableGroup[]) {
  const seen = new Set<string>()

  return groups.map(group => {
    const id = seen.has(group.id) ? crypto.randomUUID() : group.id

    seen.add(id)

    return id === group.id ? group : { ...group, id }
  })
}

export class Board {
  selected = $state<string | null>(null)
  needle = $state("")
  table = $state<string | null>(null)
  column = $state(-1)
  hover = $state<string | null>(null)
  picked = $state<string[]>([])
  groups = $state<TableGroup[]>([])
  onopen = $state<((table: string) => void) | null>(null)
  ondefine = $state<((table: string) => void) | null>(null)
  rename = $state<((id: string, name: string) => void) | null>(null)
  ungroup = $state<((id: string) => void) | null>(null)
  spots = $state<Record<string, Spot>>({})

  at(table: string, column: number) {
    return this.table === table && this.column === column
  }

  on(table: string) {
    return this.table === table
  }

  groupOf(table: string) {
    return this.groups.find(group => group.tables.includes(table))
  }

  focus(table: string | null) {
    this.selected = table
    this.table = table
    this.column = -1
  }

  reset() {
    this.focus(null)
    this.hover = null
    this.needle = ""
    this.picked = []
    this.groups = []
    this.spots = {}
  }
}

export const board = new Board()
