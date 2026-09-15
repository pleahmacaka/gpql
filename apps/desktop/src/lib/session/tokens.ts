import type { SqlToken } from "$lib/types"

export function splitTokens(value: string, tokens: SqlToken[]) {
  const out: { text: string; kind: string }[] = []
  let at = 0

  for (const token of tokens) {
    if (token.start < at || token.end > value.length) {
      continue
    }

    if (token.start > at) {
      out.push({ text: value.slice(at, token.start), kind: "" })
    }

    out.push({ text: value.slice(token.start, token.end), kind: token.kind })
    at = token.end
  }

  out.push({ text: value.slice(at), kind: "" })

  return out
}
