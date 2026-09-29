import { highlightSql, run } from "$lib/session/commands"
import { unitsAt } from "$lib/session/tokens"

export type Span = { start: number; end: number; tone: string }

export const HEAVY = 200_000

export const TONES: Record<string, string> = {
  keyword: "k",
  conditional: "k",
  repeat: "k",
  storageclass: "k",
  include: "k",
  string: "s",
  character: "s",
  number: "n",
  float: "n",
  boolean: "n",
  function: "f",
  method: "f",
  constructor: "f",
  type: "f",
  namespace: "f",
  comment: "c",
  spell: "c",
  operator: "o",
  punctuation: "o",
  delimiter: "o",
}

export async function tokenize(source: string, dialect: string) {
  if (source.length > HEAVY) {
    return []
  }

  const tokens = await run(highlightSql(source, dialect))
  const units = unitsAt(source)

  return tokens.flatMap(token => {
    const tone = TONES[token.kind]

    return tone && token.end < units.length
      ? [{ start: units[token.start], end: units[token.end], tone }]
      : []
  })
}

function entities(text: string) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
}

function piece(text: string, tone: string) {
  if (text === "") {
    return ""
  }

  return tone === ""
    ? entities(text)
    : `<span class="${tone}">${entities(text)}</span>`
}

function marked(
  text: string,
  from: number,
  tone: string,
  mark: { start: number; end: number } | null,
) {
  const to = from + text.length

  if (!mark || mark.end <= from || mark.start >= to) {
    return piece(text, tone)
  }

  const head = Math.max(mark.start, from) - from
  const tail = Math.min(mark.end, to) - from
  const inner = tone === "" ? "m" : `${tone} m`

  return (
    piece(text.slice(0, head), tone) +
    piece(text.slice(head, tail), inner) +
    piece(text.slice(tail), tone)
  )
}

export function paint(
  text: string,
  spans: Span[],
  mark: { start: number; end: number } | null = null,
) {
  let out = ""
  let at = 0

  for (const span of spans) {
    if (span.start < at || span.end > text.length) {
      continue
    }

    out += marked(text.slice(at, span.start), at, "", mark)
    out += marked(text.slice(span.start, span.end), span.start, span.tone, mark)
    at = span.end
  }

  return out + marked(text.slice(at), at, "", mark)
}

export function shifted(text: string, from: { source: string; spans: Span[] }) {
  if (from.source === text) {
    return from.spans
  }

  const old = from.source
  const room = Math.min(old.length, text.length)
  let head = 0

  while (head < room && old[head] === text[head]) {
    head++
  }

  let tail = 0

  while (
    tail < room - head &&
    old[old.length - 1 - tail] === text[text.length - 1 - tail]
  ) {
    tail++
  }

  const delta = text.length - old.length
  const cut = old.length - tail

  return from.spans.flatMap(span => {
    if (span.end <= head) {
      return [span]
    }

    if (span.start >= cut) {
      return [{ ...span, start: span.start + delta, end: span.end + delta }]
    }

    return []
  })
}
