import type { TableGroup } from "@gpql/ui"
import { generateObject, generateText, NoObjectGeneratedError } from "ai"
import { z } from "zod"

import type { Provider, TableSchema } from "$lib/types"

import { modelOf } from "./agent"

const SHAPE = z.object({
  groups: z.array(
    z.object({
      name: z.string().describe("two or three words, lower case"),
      tables: z.array(z.string()),
    }),
  ),
})

type Shape = z.infer<typeof SHAPE>

function carve(text: string) {
  const start = text.indexOf("{")
  const end = text.lastIndexOf("}")

  return start === -1 || end <= start ? "" : text.slice(start, end + 1)
}

const BRIEF = `You sort database tables into groups a person would draw on a
whiteboard. Group by what the tables are for, not by name prefix: a customer
area, a billing area, an audit area. Every table you name must come from the
list you are given. Leave a table out rather than forcing it somewhere. Never
put one table in two groups, and skip groups that would hold a single table.`

function sketch(tables: TableSchema[]) {
  return tables
    .map(table => {
      const columns = table.columns
        .map(column =>
          column.references
            ? `${column.name} -> ${column.references}`
            : column.name,
        )
        .join(", ")

      return `${table.name}: ${columns || "no columns read"}`
    })
    .join("\n")
}

export async function suggestGroups(
  tables: TableSchema[],
  provider: Provider,
  abortSignal?: AbortSignal,
): Promise<TableGroup[]> {
  if (tables.length < 3) {
    return []
  }

  const model = modelOf(provider)
  const prompt = sketch(tables)

  let groups: Shape["groups"] = []

  try {
    const answer = await generateObject({
      model,
      schema: SHAPE,
      system: BRIEF,
      prompt,
      repairText: async ({ text }) => carve(text) || null,
      abortSignal,
    })

    groups = answer.object.groups
  } catch (failure) {
    if (!NoObjectGeneratedError.isInstance(failure)) {
      throw failure
    }

    // some models ignore the schema and answer in prose; ask again in plain
    // text and read the JSON out of whatever comes back
    const spoken = await generateText({
      model,
      system: `${BRIEF}

Answer with JSON only, shaped as
{"groups":[{"name":"...","tables":["...","..."]}]}. No prose, no code fences.`,
      prompt,
      abortSignal,
    })

    const read = SHAPE.safeParse(JSON.parse(carve(spoken.text) || "{}"))

    if (!read.success) {
      throw new Error("the model did not answer with a grouping")
    }

    groups = read.data.groups
  }

  const known = new Set(tables.map(table => table.name))
  const taken = new Set<string>()

  return groups
    .map((group, index) => ({
      id: `ai${index + 1}`,
      name: group.name.trim() || `group ${index + 1}`,
      tables: group.tables.filter(name => {
        if (!known.has(name) || taken.has(name)) {
          return false
        }

        taken.add(name)

        return true
      }),
    }))
    .filter(group => group.tables.length > 1)
}
