import type { SqlToken } from "$lib/types"

function utf8Width(code: number) {
  if (code < 0x80) {
    return 1
  }

  if (code < 0x800) {
    return 2
  }

  return code < 0x10000 ? 3 : 4
}

// tree-sitter reports utf-8 byte offsets while strings index utf-16 units
export function unitsAt(value: string) {
  const units: number[] = []
  let unit = 0

  for (const char of value) {
    const width = utf8Width(char.codePointAt(0) ?? 0)

    for (let step = 0; step < width; step++) {
      units.push(unit)
    }

    unit += char.length
  }

  units.push(unit)

  return units
}

export function unitColumn(line: string, byteColumn: number) {
  const units = unitsAt(line)

  return units[Math.min(byteColumn, units.length - 1)]
}

export function splitTokens(value: string, tokens: SqlToken[]) {
  const units = unitsAt(value)
  const out: { text: string; kind: string }[] = []
  let at = 0

  for (const token of tokens) {
    if (token.end >= units.length) {
      continue
    }

    const start = units[token.start]
    const end = units[token.end]

    if (start < at) {
      continue
    }

    if (start > at) {
      out.push({ text: value.slice(at, start), kind: "" })
    }

    out.push({ text: value.slice(start, end), kind: token.kind })
    at = end
  }

  out.push({ text: value.slice(at), kind: "" })

  return out
}
