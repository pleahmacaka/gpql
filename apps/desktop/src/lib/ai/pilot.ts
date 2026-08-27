import { generateText, Output } from "ai"
import { z } from "zod"

import type { Provider, TableInfo, TableSchema } from "$lib/types"

import { modelOf, outline } from "./agent"

const RULES = `You drive a database client for the user. Decide one move for the
latest request.

query: write one query that answers a data question; the app opens the query
tab with it.
data: open a table's rows. Pick the table from the list; "the biggest table"
means the highest row count.
schema: show where a table sits in the schema graph.
chat: anything else.

note is ONE short sentence, like a voice assistant: "3일치 조회 쿼리를
준비했어요" not an explanation. Never use markdown, lists, or backticks in
note. Prefer acting (query/data/schema) over chatting; when the user asks
about the data, write the query instead of describing tables. Match the
user's language in note. SQL must fit the schema below.`

const note = z.string().describe("one short sentence in the user's language")

const MOVE = z.discriminatedUnion("go", [
  z.object({ go: z.literal("query"), sql: z.string(), note }),
  z.object({ go: z.literal("data"), table: z.string(), note }),
  z.object({ go: z.literal("schema"), table: z.string(), note }),
  z.object({ go: z.literal("chat"), note }),
])

export type PilotMove = z.infer<typeof MOVE>

export async function pilot(
  provider: Provider,
  history: { role: "you" | "agent"; text: string }[],
  schema: TableSchema[],
  tables: TableInfo[],
): Promise<PilotMove> {
  const counts = tables
    .map(table => `${table.name}: ${table.rows} rows`)
    .join("\n")

  const answer = await generateText({
    model: modelOf(provider),
    output: Output.object({ schema: MOVE }),
    system: `${RULES}\n\nSchema:\n${outline(schema)}\n\nRow counts:\n${counts}`,
    messages: history.map(turn => ({
      role: turn.role === "you" ? ("user" as const) : ("assistant" as const),
      content: turn.text,
    })),
  })

  return answer.output
}
