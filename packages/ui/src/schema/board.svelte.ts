export type TableGroup = {
  id: string
  name: string
  tables: string[]
}

export type Spot = { x: number; y: number }

export class Board {
  selected = $state<string | null>(null)
  table = $state<string | null>(null)
  column = $state(-1)
  picked = $state<string[]>([])
  groups = $state<TableGroup[]>([])
  onopen = $state<((table: string) => void) | null>(null)
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
}

export const board = new Board()
