import { generateText } from "ai"

import type { Provider, TableSchema } from "$lib/types"

import { modelOf, outline } from "./agent"

const RULES = `You turn a request into one SQL statement for the schema you are
given. Answer with the statement only: no prose, no code fences. Prefer
read-only statements. When a statement is already there, edit that one instead
of starting over.`

export async function writeSql(
  provider: Provider,
  prompt: string,
  tables: TableSchema[],
  base = "",
) {
  const current = base.trim()

  const answer = await generateText({
    model: modelOf(provider),
    system: `${RULES}\n\nSchema:\n${outline(tables)}`,
    prompt:
      current === "" ? prompt : `${prompt}\n\nStatement so far:\n${current}`,
  })

  return answer.text
    .replace(/^```(?:sql)?\s*/i, "")
    .replace(/```$/, "")
    .trim()
}
