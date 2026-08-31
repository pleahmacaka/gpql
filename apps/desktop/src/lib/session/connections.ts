import type { recent } from "$lib/db/schema"

export type Connection = typeof recent.$inferSelect

export type Section = { folder: string | null; entries: Connection[] }

export function foldersOf(entries: Connection[]) {
  return [
    ...new Set(
      entries
        .map(entry => entry.folder)
        .filter(name => name !== null && name.trim() !== ""),
    ),
  ] as string[]
}

// grouped connections come first so the loose ones read as the remainder
export function sectionsOf(entries: Connection[]): Section[] {
  const loose = entries.filter(entry => !entry.folder)
  const named = foldersOf(entries).map(folder => ({
    folder,
    entries: entries.filter(entry => entry.folder === folder),
  }))

  return loose.length > 0 ? [...named, { folder: null, entries: loose }] : named
}

export function reorder(order: string[], from: string, to: string) {
  const moving = order.indexOf(from)
  const landing = order.indexOf(to)

  if (from === to || moving === -1 || landing === -1) {
    return order
  }

  const next = [...order]

  next.splice(moving, 1)
  next.splice(landing, 0, from)

  return next
}

export function shiftBy(order: string[], url: string, delta: number) {
  const at = order.indexOf(url)
  const landing = at + delta

  if (at === -1 || landing < 0 || landing >= order.length) {
    return order
  }

  return reorder(order, url, order[landing])
}

export function toTop(order: string[], url: string) {
  return order[0] ? reorder(order, url, order[0]) : order
}
