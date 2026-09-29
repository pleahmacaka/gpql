import { type Grid, type Put, type Scene, shade } from "./field"

type Cell = { col: number; row: number; char: string }

const hash = (seed: number) => {
  const value = Math.sin(seed * 12.9898) * 43758.5453

  return value - Math.floor(value)
}

const glow = (distance: number, spread: number) =>
  Math.exp(-((distance / spread) ** 2))

function frame(grid: Grid, wide: number, tall: number) {
  const width = Math.min(grid.cols - 2, wide)
  const height = Math.min(grid.rows - 2, tall)

  return {
    width,
    height,
    left: Math.floor((grid.cols - width) / 2),
    top: Math.floor((grid.rows - height) / 2),
  }
}

export function sheet(): Scene {
  let box = { width: 0, height: 0, left: 0, top: 0 }
  let stops: number[] = []

  const layout = (grid: Grid) => {
    box = frame(grid, 48, 12)

    const shares = [0.3, 0.22, 0.28]
    let at = box.left

    stops = [at]

    for (const share of shares) {
      at += Math.round(share * box.width)
      stops.push(at)
    }

    stops.push(box.left + box.width - 1)
  }

  const paint: Scene["paint"] = (put, _grid, time) => {
    const { left, top, width, height } = box
    const bottom = top + height - 1
    const scan = top + ((time * 2.4) % (height + 8)) - 4

    for (let row = top; row <= bottom; row++) {
      const lit = glow(row - scan, 1.4)
      const rule = row === top || row === top + 2 || row === bottom

      for (let col = left; col < left + width; col++) {
        const stop = stops.indexOf(col)

        if (rule) {
          put(col, row, stop === -1 ? "-" : "+", 0.26 + lit * 0.3)
          continue
        }

        if (stop !== -1) {
          put(col, row, "|", 0.26 + lit * 0.3)
          continue
        }

        const column = stops.findIndex(edge => edge > col) - 1
        const start = stops[column] + 2
        const span = stops[column + 1] - start - 1
        const reach = 0.25 + 0.6 * hash(row * 31 + column * 7)

        if (
          col < start ||
          col - start >= span * (row === top + 1 ? 0.7 : reach)
        ) {
          continue
        }

        if (row === top + 1) {
          put(col, row, "#", 0.4 + lit * 0.4, lit > 0.5 ? "primary" : "ink")
          continue
        }

        const level = 0.12 + lit * 0.6

        put(
          col,
          row,
          lit > 0.3 ? shade(level + 0.2) : ".",
          0.3 + lit * 0.5,
          lit > 0.5 ? "primary" : "ink",
        )
      }
    }
  }

  return { layout, paint }
}

export function graph(): Scene {
  let boxes: Cell[] = []
  let heads: Cell[] = []
  let edges: Cell[][] = []

  const outline = (left: number, top: number, wide: number, tall: number) => {
    for (let row = top; row < top + tall; row++) {
      for (let col = left; col < left + wide; col++) {
        const edgeRow = row === top || row === top + tall - 1
        const edgeCol = col === left || col === left + wide - 1

        if (edgeRow && edgeCol) {
          boxes.push({ col, row, char: "+" })
        } else if (edgeRow) {
          boxes.push({ col, row, char: "-" })
        } else if (edgeCol) {
          boxes.push({ col, row, char: "|" })
        } else if (row === top + 1) {
          heads.push({ col, row, char: col < left + wide * 0.7 ? "=" : " " })
        } else if (col < left + 2 + ((row * 5 + left) % (wide - 4))) {
          boxes.push({ col, row, char: "." })
        }
      }
    }
  }

  const route = (from: Cell, to: Cell) => {
    const path: Cell[] = []
    const bend = Math.round((from.col + to.col) / 2)
    const step = to.row > from.row ? 1 : -1

    for (let col = from.col; col <= bend; col++) {
      path.push({ col, row: from.row, char: "-" })
    }

    for (let row = from.row + step; row !== to.row; row += step) {
      path.push({ col: bend, row, char: "|" })
    }

    for (let col = bend; col <= to.col; col++) {
      path.push({ col, row: to.row, char: col === bend ? "+" : "-" })
    }

    return path
  }

  const layout = (grid: Grid) => {
    const box = frame(grid, 52, 15)
    const wide = Math.max(8, Math.floor(box.width * 0.3))
    const tall = Math.max(4, Math.min(6, Math.floor(box.height / 2.5)))
    const right = box.left + box.width - wide
    const lower = box.top + box.height - tall
    const middle = box.top + Math.floor((box.height - tall) / 2)

    boxes = []
    heads = []
    outline(box.left, middle, wide, tall)
    outline(right, box.top, wide, tall)
    outline(right, lower, wide, tall)

    const exit = { col: box.left + wide, row: middle + 2, char: "" }

    edges = [
      route(exit, { col: right - 1, row: box.top + 2, char: "" }),
      route(exit, { col: right - 1, row: lower + 2, char: "" }),
    ]
  }

  const paint: Scene["paint"] = (put: Put, _grid, time) => {
    for (const cell of boxes) {
      put(cell.col, cell.row, cell.char, cell.char === "." ? 0.26 : 0.4)
    }

    for (const cell of heads) {
      put(cell.col, cell.row, cell.char, 0.55, "primary")
    }

    edges.forEach((path, index) => {
      const head = (time * 7 + index * 11) % (path.length + 12)

      path.forEach((cell, at) => {
        const lit = at <= head ? glow(head - at, 3) : 0

        put(
          cell.col,
          cell.row,
          lit > 0.6 ? "*" : cell.char,
          0.22 + lit * 0.7,
          lit > 0.3 ? "primary" : "ink",
        )
      })
    })
  }

  return { layout, paint }
}

export function link(): Scene {
  let left = 0
  let right = 0
  let row = 0
  let size = 0

  const layout = (grid: Grid) => {
    const box = frame(grid, 44, 7)

    size = Math.max(3, Math.min(7, Math.floor(box.width / 6)))
    left = box.left
    right = box.left + box.width - size
    row = Math.floor(grid.rows / 2)
  }

  const node = (
    put: Put,
    from: number,
    time: number,
    tone: "ink" | "primary",
  ) => {
    for (let dy = -1; dy <= 1; dy++) {
      for (let col = from; col < from + size; col++) {
        const edge = dy !== 0 || col === from || col === from + size - 1
        const pulse = 0.5 + 0.5 * Math.sin(time * 3 + col)

        put(
          col,
          row + dy,
          edge ? (dy === 0 ? "|" : "-") : shade(0.4 + pulse * 0.5),
          edge ? 0.5 : 0.6,
          edge ? "ink" : tone,
        )
      }
    }
  }

  const paint: Scene["paint"] = (put, _grid, time) => {
    const start = left + size + 1
    const end = right - 2
    const span = Math.max(1, end - start)

    node(put, left, time, "primary")
    node(put, right, time + 1.5, "ink")

    for (let col = start; col <= end; col++) {
      const out = (((col - start - time * 9) % 6) + 6) % 6
      const back = (((col - start + time * 6) % 9) + 9) % 9

      put(col, row - 1, out < 1 ? ">" : ".", out < 1 ? 0.85 : 0.2, "primary")
      put(col, row + 1, back < 1 ? "<" : ".", back < 1 ? 0.5 : 0.14)
    }

    put(start + ((time * 4) % span), row, ":", 0.3)
  }

  return { layout, paint }
}
