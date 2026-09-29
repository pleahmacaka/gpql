import { json } from "@sveltejs/kit"

import { auth } from "./auth"

export class Refusal extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message)
  }
}

export async function answer(
  run: () => Promise<Response>,
  headers: Record<string, string> = {},
) {
  const response = await run().catch(problem => {
    if (problem instanceof Refusal) {
      return json({ message: problem.message }, { status: problem.status })
    }

    console.error(problem)

    return json(
      { message: "the server could not finish that request" },
      { status: 500 },
    )
  })

  for (const [name, value] of Object.entries(headers)) {
    response.headers.set(name, value)
  }

  return response
}

export async function readJson(request: Request, limit: number) {
  const declared = Number(request.headers.get("content-length") ?? 0)

  if (declared > limit) {
    throw new Refusal(413, "the request body is too large")
  }

  const bytes = await request.arrayBuffer()

  if (bytes.byteLength > limit) {
    throw new Refusal(413, "the request body is too large")
  }

  try {
    return JSON.parse(new TextDecoder().decode(bytes)) as unknown
  } catch {
    throw new Refusal(400, "the request body is not JSON")
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

export async function viewer(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers })

  return session?.user ?? null
}

export async function signedIn(request: Request) {
  const user = await viewer(request)

  if (!user) {
    throw new Refusal(401, "sign in first")
  }

  return user
}
