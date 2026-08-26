import { generateText } from "ai"

import type { Provider, TableSchema } from "$lib/types"

import { modelOf, outline } from "./agent"

const RULES = `You help someone read and shape a database they are connected to.
Answer in the language of the question. Keep answers short, and when SQL helps,
show it in a fenced block.`

export async function talk(
  provider: Provider,
  history: { role: "you" | "agent"; text: string }[],
  tables: TableSchema[],
) {
  const answer = await generateText({
    model: modelOf(provider),
    system: `${RULES}\n\nSchema:\n${outline(tables)}`,
    messages: history.map(turn => ({
      role: turn.role === "you" ? ("user" as const) : ("assistant" as const),
      content: turn.text,
    })),
  })

  return answer.text
}
