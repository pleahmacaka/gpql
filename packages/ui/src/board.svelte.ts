export class Board {
  selected = $state<string | null>(null)
  table = $state<string | null>(null)
  column = $state(-1)

  at(table: string, column: number) {
    return this.table === table && this.column === column
  }

  on(table: string) {
    return this.table === table
  }
}

export const board = new Board()
