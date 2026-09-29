type Scanned = { text: string; exact: boolean }

const NUMBER = "+-.eE0123456789"

function stringEnd(text: string, start: number) {
  let at = start + 1

  while (at < text.length && text[at] !== '"') {
    at += text[at] === "\\" ? 2 : 1
  }

  return at + 1
}

function structured(text: string) {
  const head = text.trimStart()[0]

  if (head !== "{" && head !== "[") {
    return false
  }

  try {
    JSON.parse(text)

    return true
  } catch {
    return false
  }
}

function minify(json: string): Scanned {
  let text = ""
  let exact = true
  let at = 0

  while (at < json.length) {
    const char = json[at]

    if (char === '"') {
      const end = stringEnd(json, at)

      text += json.slice(at, end)
      at = end
    } else if (char === "-" || (char >= "0" && char <= "9")) {
      let end = at

      while (end < json.length && NUMBER.includes(json[end])) {
        end++
      }

      const token = json.slice(at, end)

      exact &&= JSON.stringify(Number(token)) === token
      text += token
      at = end
    } else {
      if (char.trim() !== "") {
        text += char
      }

      at++
    }
  }

  return { text, exact }
}

function layout(compact: string, lines: boolean) {
  let text = ""
  let depth = 0
  let at = 0

  const pad = () => (lines ? `\n${"  ".repeat(depth)}` : "")

  while (at < compact.length) {
    const char = compact[at]
    const next = compact[at + 1]

    if (char === '"') {
      const end = stringEnd(compact, at)

      text += compact.slice(at, end)
      at = end

      continue
    }

    if ((char === "{" && next === "}") || (char === "[" && next === "]")) {
      text += char + next
      at += 2

      continue
    }

    if (char === "{" || char === "[") {
      depth++
      text += char + pad()
    } else if (char === "}" || char === "]") {
      depth--
      text += pad() + char
    } else if (char === ",") {
      text += `,${lines ? pad() : " "}`
    } else if (char === ":") {
      text += ": "
    } else {
      text += char
    }

    at++
  }

  return text
}

function same(left: unknown, right: unknown): boolean {
  if (left === right) {
    return true
  }

  if (
    typeof left !== "object" ||
    typeof right !== "object" ||
    left === null ||
    right === null ||
    Array.isArray(left) !== Array.isArray(right)
  ) {
    return false
  }

  const a = left as Record<string, unknown>
  const b = right as Record<string, unknown>
  const keys = Object.keys(a)

  return (
    keys.length === Object.keys(b).length &&
    keys.every(key => Object.hasOwn(b, key) && same(a[key], b[key]))
  )
}

export function looksStructured(value: string | null) {
  return value !== null && structured(value)
}

export function pretty(value: string, lines = true) {
  return structured(value) ? layout(minify(value).text, lines) : value
}

export function settle(
  original: string | null,
  draft: string,
  formatted: boolean,
): string | undefined {
  if (draft === original) {
    return undefined
  }

  if (
    !formatted ||
    original === null ||
    !structured(original) ||
    !structured(draft)
  ) {
    return draft
  }

  const before = minify(original)
  const after = minify(draft)

  if (before.text === after.text) {
    return undefined
  }

  if (
    before.exact &&
    after.exact &&
    same(JSON.parse(original), JSON.parse(draft))
  ) {
    return undefined
  }

  if (original === before.text) {
    return after.text
  }

  return original === layout(before.text, false)
    ? layout(after.text, false)
    : draft
}
