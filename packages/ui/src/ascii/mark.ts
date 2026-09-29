import { CUBE, GLYPH } from "../controls/mark"
import { type Grid, type Scene, shade, type Tone } from "./field"

type Kind = "face" | "edge" | "side" | "cube"

type Cell = {
  col: number
  row: number
  kind: Kind
  depth: number
  cover: number
  slant: number
  at: number
}

type Lane = {
  id: number
  row: number
  from: number
  to: number
  tape: string
  speed: number
  inward: 1 | -1
}

type MarkOptions = {
  streams?: boolean
  lanes?: number[] | { left: number[]; right: number[] }
  hot?: () => number | null
  start?: number
  assemble?: number
  busy?: () => boolean
}

const SIZE = 884
const ORIGIN = { x: 188, y: 180 }
const DEPTH = 3
const NOISE = "GPQL01#%*+=:"
const SETTLE = 0.2

const TAPES = [
  "select topic, count(*) from message group by topic",
  "MATCH (a:Account)-[:SENT]->(m) RETURN a.handle",
  'from(bucket: "sensors") |> range(start: -1h)',
  'publish sensors/attic {"temp":21.4}',
  "get logs/2026/09/29/app.jsonl",
  "explain analyze select * from pin",
]

function coverage(grid: Grid, box: { x: number; y: number; size: number }) {
  const probe = document.createElement("canvas")
  const context = probe.getContext("2d", { willReadFrequently: true })

  if (!context || grid.cols === 0 || grid.rows === 0) {
    return null
  }

  probe.width = grid.cols
  probe.height = grid.rows

  const scale = box.size / SIZE

  const paint = (paths: string[], width: number) => {
    context.clearRect(0, 0, grid.cols, grid.rows)
    context.setTransform(
      scale / grid.cellWidth,
      0,
      0,
      scale / grid.cellHeight,
      (box.x - ORIGIN.x * scale) / grid.cellWidth,
      (box.y - ORIGIN.y * scale) / grid.cellHeight,
    )
    context.lineWidth = width
    context.lineCap = "round"
    context.lineJoin = "round"
    context.strokeStyle = "#000"

    for (const path of paths) {
      context.stroke(new Path2D(path))
    }

    context.setTransform(1, 0, 0, 1, 0, 0)

    const { data } = context.getImageData(0, 0, grid.cols, grid.rows)

    return (col: number, row: number) =>
      col < 0 || row < 0 || col >= grid.cols || row >= grid.rows
        ? 0
        : data[(row * grid.cols + col) * 4 + 3] / 255
  }

  const glyph = paint(GLYPH, 64)
  const glyphAt = new Float32Array(grid.cols * grid.rows)

  for (let row = 0; row < grid.rows; row++) {
    for (let col = 0; col < grid.cols; col++) {
      glyphAt[row * grid.cols + col] = glyph(col, row)
    }
  }

  const cube = paint(CUBE, 16)

  const face = (col: number, row: number) =>
    col < 0 || row < 0 || col >= grid.cols || row >= grid.rows
      ? 0
      : glyphAt[row * grid.cols + col]

  return { face, cube }
}

export function mark(options: MarkOptions = {}): Scene {
  const {
    streams = false,
    lanes: pinned,
    hot,
    start = 0,
    assemble = 1.6,
    busy,
  } = options

  const fixed = Array.isArray(pinned) ? { left: pinned, right: pinned } : pinned

  const flowing = streams || fixed !== undefined

  let cells: Cell[] = []
  let lanes: Lane[] = []

  const layout = (grid: Grid) => {
    const { width, height, cellHeight } = grid
    const size = Math.min(
      height * (fixed ? 0.78 : 0.88),
      width * (fixed ? 0.6 : flowing ? 0.56 : 0.9),
    )
    const x = (width - size) / 2
    const y = (height - size) / 2
    const found = coverage(grid, { x, y, size })

    cells = []
    lanes = []

    if (!found) {
      return
    }

    const { face, cube } = found

    for (let row = 0; row < grid.rows; row++) {
      for (let col = 0; col < grid.cols; col++) {
        const cover = face(col, row)
        const at = Math.random() * 0.55 + ((grid.rows - row) / grid.rows) * 0.4
        const slant = (col / grid.cols + row / grid.rows) / 2

        if (cover > 0.35) {
          const edge =
            face(col - 1, row - 1) < 0.35 || face(col, row - 1) < 0.35

          cells.push({
            col,
            row,
            kind: edge ? "edge" : "face",
            depth: 0,
            cover,
            slant,
            at,
          })
          continue
        }

        const depth = [1, 2, 3].find(
          step => face(col - step, row - step) > 0.35,
        )

        if (depth !== undefined && depth <= DEPTH) {
          cells.push({ col, row, kind: "side", depth, cover, slant, at })
          continue
        }

        if (cube(col, row) > 0.3) {
          cells.push({ col, row, kind: "cube", depth: 0, cover, slant, at })
        }
      }
    }

    if (!flowing) {
      return
    }

    const top = Math.floor(y / cellHeight)
    const span = Math.floor(size / cellHeight)
    const inner = Math.floor(x / grid.cellWidth)
    const outer = Math.ceil((x + size) / grid.cellWidth)

    const lane = (id: number, row: number, left: boolean): Lane[] => {
      const across = cells.filter(cell => cell.row === row)
      const edge = left
        ? across.reduce((low, cell) => Math.min(low, cell.col), inner)
        : across.reduce((high, cell) => Math.max(high, cell.col), outer)
      const room = left ? edge : grid.cols - 1 - edge

      if (room < 4) {
        return []
      }

      return [
        {
          id,
          row,
          from: left ? 0 : edge + 2,
          to: left ? edge - 2 : grid.cols - 1,
          tape: `${TAPES[id % TAPES.length]}      `,
          speed: 2 + ((id * 7) % 5) * 0.5,
          inward: left ? 1 : -1,
        },
      ]
    }

    lanes = fixed
      ? [
          ...fixed.left.flatMap((share, index) =>
            lane(index, Math.floor(share * grid.rows), true),
          ),
          ...fixed.right.flatMap((share, index) =>
            lane(
              index + fixed.left.length,
              Math.floor(share * grid.rows),
              false,
            ),
          ),
        ]
      : TAPES.flatMap((_, index) =>
          lane(
            index,
            top + Math.floor(((index + 0.7) / TAPES.length) * span),
            index % 2 === 0,
          ),
        )
  }

  const paint: Scene["paint"] = (put, _grid, time) => {
    const progress = (time - start) / assemble
    const rush = busy?.() ? 2.6 : 1
    const band = ((time * 0.07 * rush) % 1.5) - 0.25

    for (const cell of cells) {
      if (progress < cell.at - SETTLE) {
        continue
      }

      if (progress < cell.at) {
        const noise = NOISE[Math.floor(Math.random() * NOISE.length)]

        put(cell.col, cell.row, noise, 0.5, "primary")
        continue
      }

      const glow = Math.exp(-(((cell.slant - band) / 0.05) ** 2))
      const tone: Tone = glow > 0.45 ? "primary" : "ink"

      if (cell.kind === "edge") {
        put(cell.col, cell.row, glow > 0.2 ? "@" : "#", 0.8, tone)
      } else if (cell.kind === "face") {
        const level = 0.55 + 0.3 * cell.cover + glow * 0.2

        put(cell.col, cell.row, shade(level), 0.55 + 0.35 * level, tone)
      } else if (cell.kind === "side") {
        const level = 0.5 - cell.depth * 0.12 + glow * 0.15

        put(cell.col, cell.row, shade(level), 0.5 - cell.depth * 0.1, tone)
      } else {
        put(cell.col, cell.row, ".", 0.35 + glow * 0.3, tone)
      }
    }

    const lit = hot?.() ?? null

    for (const lane of lanes) {
      const length = lane.tape.length
      const reach = Math.max(1, lane.to - lane.from)
      const heat = lane.id === lit
      const pace = lane.speed * rush * (heat ? 4 : 1)

      for (let col = lane.from; col <= lane.to; col++) {
        const index = Math.floor(col - lane.inward * time * pace)
        const char = lane.tape[((index % length) + length) % length]
        const out = lane.inward === 1 ? col - lane.from : lane.to - col
        const near = out / reach
        const alpha = heat
          ? 0.45 + 0.5 * near
          : Math.min(1, out / 8) * (0.08 + 0.3 * near ** 1.6)

        put(col, lane.row, char, alpha, heat || near > 0.95 ? "primary" : "ink")
      }
    }
  }

  return { layout, paint }
}
