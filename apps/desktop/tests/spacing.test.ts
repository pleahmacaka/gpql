import { expect, test } from "bun:test"
import { readdirSync, readFileSync, statSync } from "node:fs"
import { join } from "node:path"

const SPACING =
  "p|px|py|pt|pr|pb|pl|m|mx|my|mt|mr|mb|ml|gap|gap-x|gap-y" +
  "|space-x|space-y|size|w|h|min-w|min-h|max-w|max-h" +
  "|inset|inset-x|inset-y|top|right|bottom|left"

const offGrid = new RegExp(`\\b-?(?:${SPACING})-\\d+\\.5\\b`, "g")

const arbitrary = /\b(?:p|px|py|pt|pr|pb|pl|m|mx|my|gap|size|w|h)-\[/g

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

    return name.endsWith(".svelte") || name.endsWith(".ts") ? [path] : []
  })
}

const files = roots.flatMap(sources)

test("there is something to check", () => {
  expect(files.length).toBeGreaterThan(50)
})

test("every spacing utility sits on the four unit grid", () => {
  const strays: string[] = []

  for (const path of files) {
    for (const hit of readFileSync(path, "utf8").matchAll(offGrid)) {
      strays.push(`${path}: ${hit[0]}`)
    }
  }

  expect(strays).toEqual([])
})

test("no spacing is written as an arbitrary value", () => {
  const strays: string[] = []

  for (const path of files) {
    for (const hit of readFileSync(path, "utf8").matchAll(arbitrary)) {
      strays.push(`${path}: ${hit[0]}`)
    }
  }

  expect(strays).toEqual([])
})

test("no length is spelled out in pixels", () => {
  const strays: string[] = []

  for (const path of files) {
    for (const hit of readFileSync(path, "utf8").matchAll(/\b\d+px\b/g)) {
      strays.push(`${path}: ${hit[0]}`)
    }
  }

  expect(strays).toEqual([])
})
