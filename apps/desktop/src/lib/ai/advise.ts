import { generateText, Output } from "ai"
import { z } from "zod"

import type { Plan, Provider, TableSchema } from "$lib/types"

import { modelOf, outline } from "./agent"

const ADVICE = z.object({
  verdict: z.string().describe("one sentence on what makes this query slow"),
  steps: z
    .array(
      z.object({
        why: z.string().describe("what this fixes, one short sentence"),
        sql: z
          .string()
          .describe("the statement to run, or an empty string if none applies"),
      }),
    )
    .describe("ordered, most valuable first"),
})

export type Advice = z.infer<typeof ADVICE>

const DIAGNOSE = `You are reading a real execution plan from a database the user
is connected to. Say plainly what costs the time, then give the smallest set of
changes that would help, most valuable first.

Prefer an index only when the plan shows a scan that an index would actually
remove; say so when a rewrite beats an index. Never invent columns that are not
in the schema. Keep every sentence short and in the user's language. When a step
needs no statement, leave sql empty.`

export async function diagnose(
  provider: Provider,
  sql: string,
  plan: Plan,
  schema: TableSchema[],
): Promise<Advice> {
  const answer = await generateText({
    model: modelOf(provider),
    output: Output.object({ schema: ADVICE }),
    system: `${DIAGNOSE}\n\nSchema:\n${outline(schema)}`,
    prompt: `Query:\n${sql}\n\nPlan:\n${plan.text}`,
  })

  return answer.output
}

const NOTES = z.object({
  tables: z.array(
    z.object({
      name: z.string(),
      note: z.string().describe("one short sentence on what this table holds"),
    }),
  ),
})

const DESCRIBE = `Describe what each table is for, based on its name, columns and
foreign keys. One short sentence each, in the user's language. Say only what the
schema supports; do not guess at business rules that are not visible.`

export async function describeTables(
  provider: Provider,
  schema: TableSchema[],
  abortSignal?: AbortSignal,
): Promise<Record<string, string>> {
  const answer = await generateText({
    model: modelOf(provider),
    output: Output.object({ schema: NOTES }),
    system: DESCRIBE,
    prompt: outline(schema),
    abortSignal,
  })

  return Object.fromEntries(
    answer.output.tables.map(entry => [entry.name, entry.note]),
  )
}
