import { generateText } from "ai"

import type { Provider } from "$lib/types"

import { modelOf } from "./agent"

export async function entitle(
  provider: Provider,
  history: { role: "you" | "agent"; text: string }[],
) {
  const answer = await generateText({
    model: modelOf(provider),
    system:
      "Give this conversation a title of at most five words, in its own " +
      "language. Answer with the title only, no quotes.",
    messages: [
      {
        role: "user" as const,
        content: history.map(turn => `${turn.role}: ${turn.text}`).join("\n"),
      },
    ],
  })

  return answer.text.trim().replace(/^["']|["']$/g, "")
}
