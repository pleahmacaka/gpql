import type { Scene } from "@gpql/ui"

type WireOptions = { sealed?: boolean; pace?: number }

const wrap = (value: number, size: number) => ((value % size) + size) % size

export function wire({ sealed = false, pace = 8 }: WireOptions = {}): Scene {
  let row = 0
  let cols = 0

  const layout: Scene["layout"] = grid => {
    row = Math.floor(grid.rows / 2)
    cols = grid.cols
  }

  const paint: Scene["paint"] = (put, _grid, time) => {
    for (let col = 0; col < cols; col++) {
      const out = wrap(col - time * pace, 9)
      const back = wrap(col + time * pace * 0.55, 14)

      if (out < 1) {
        put(col, row, ">", 0.95, "primary")
      } else if (out < 2) {
        put(col, row, sealed ? "=" : "-", 0.6, "primary")
      } else if (back < 1) {
        put(col, row, "<", 0.5)
      } else {
        put(col, row, sealed ? "=" : "-", sealed ? 0.4 : 0.22)
      }
    }
  }

  return { layout, paint }
}
