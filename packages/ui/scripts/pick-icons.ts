import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs"
import { join } from "node:path"

const ROOTS = [
  "apps/desktop/src",
  "apps/desktop/src-tauri/src",
  "apps/web/src",
  "packages/ui/src",
]

const PACKS = ["lucide", "simple-icons"] as const

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap(entry => {
    const path = join(dir, entry)

    if (entry === "node_modules" || entry === "paraglide" || entry === "packs") {
      return []
    }

    return statSync(path).isDirectory() ? walk(path) : [path]
  })
}

const sources = ROOTS.flatMap(root => walk(root))
  .filter(path => /\.(svelte|ts|js|rs)$/.test(path))
  .map(path => readFileSync(path, "utf8"))
  .join("\n")

for (const pack of PACKS) {
  const wanted = new Set(
    [...sources.matchAll(new RegExp(`${pack}:([a-z0-9-]+)`, "g"))].map(
      hit => hit[1],
    ),
  )

  const whole = JSON.parse(
    readFileSync(
      `packages/ui/node_modules/@iconify-json/${pack}/icons.json`,
      "utf8",
    ),
  ) as {
    icons: Record<string, unknown>
    aliases?: Record<string, { parent: string }>
  }

  const icons: Record<string, unknown> = {}
  const aliases: Record<string, { parent: string }> = {}

  for (const name of wanted) {
    if (whole.icons[name]) {
      icons[name] = whole.icons[name]
      continue
    }

    const alias = whole.aliases?.[name]

    if (alias) {
      aliases[name] = alias
      icons[alias.parent] = whole.icons[alias.parent]
    }
  }

  const missing = [...wanted].filter(name => !icons[name] && !aliases[name])

  writeFileSync(
    `packages/ui/src/icons/packs/${pack}.json`,
    `${JSON.stringify({ ...whole, icons, aliases })}\n`,
  )

  console.log(
    pack,
    Object.keys(icons).length,
    "icons",
    missing.length ? `missing: ${missing.join(" ")}` : "",
  )
}
