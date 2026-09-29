import type { SchemaTable } from "@gpql/ui"

const MARK = "@ref"

function refs(text: string | null | undefined) {
  const words = (text ?? "").split(/\s+/)

  return words.flatMap((word, index) => {
    const target =
      word === MARK
        ? (words[index + 1] ?? "")
        : word.startsWith(`${MARK}:`)
          ? word.slice(MARK.length + 1)
          : ""
    const clean = target.replace(/[^\w.]+$/, "")

    return clean.includes(".") ? [clean] : []
  })
}

export function withHints(tables: SchemaTable[]) {
  return tables.map(table => {
    const found = [table.note, ...table.columns.map(column => column.note)]
      .flatMap(refs)
      .filter(target => !(table.hints ?? []).includes(target))

    return found.length === 0
      ? table
      : { ...table, hints: [...new Set([...(table.hints ?? []), ...found])] }
  })
}
