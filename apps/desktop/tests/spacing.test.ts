import { expect, test } from "bun:test"
import { readdirSync, readFileSync, statSync } from "node:fs"
import { join } from "node:path"

const SPACING =
  "p|px|py|pt|pr|pb|pl|m|mx|my|mt|mr|mb|ml|gap|gap-x|gap-y" +
  "|space-x|space-y|size|w|h|min-w|min-h|max-w|max-h" +
  "|inset|inset-x|inset-y|top|right|bottom|left"

const offGrid = new RegExp(`\\b-?(?:${SPACING})-\\d+\\.5\\b`, "g")

const arbitrary = /\b[a-z][a-z0-9-]*-\[/g

const hairline = /\b(?:[a-z]+-)+px\b/g

const pixels = /\b\d+px\b/g

const roots = [
  join(import.meta.dir, "..", "src"),
  join(import.meta.dir, "..", "..", "..", "packages", "ui", "src"),
  join(import.meta.dir, "..", "..", "web", "src"),
]

function sources(folder: string): string[] {
  return readdirSync(folder).flatMap(name => {
    const path = join(folder, name)

    if (statSync(path).isDirectory()) {
      return name === "paraglide" ? [] : sources(path)
    }

    return [".svelte", ".ts", ".css"].some(kind => name.endsWith(kind))
      ? [path]
      : []
  })
}

const files = roots.flatMap(sources)

test("there is something to check", () => {
  expect(files.length).toBeGreaterThan(50)
})

function strays(pattern: RegExp) {
  return files.flatMap(path =>
    [...readFileSync(path, "utf8").matchAll(pattern)].map(
      hit => `${path}: ${hit[0]}`,
    ),
  )
}

test("every spacing utility sits on the four unit grid", () => {
  expect(strays(offGrid)).toEqual([])
})

test("no utility takes an arbitrary value", () => {
  expect(strays(arbitrary)).toEqual([])
})

test("no utility is sized in single pixels", () => {
  expect(strays(hairline)).toEqual([])
})

test("no length is spelled out in pixels", () => {
  expect(strays(pixels)).toEqual([])
})
