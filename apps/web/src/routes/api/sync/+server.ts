import { json } from "@sveltejs/kit"

import { answer, readJson, signedIn } from "$lib/server/http"
import {
  everything,
  forget,
  merge,
  readPayload,
  SYNC_BODY_LIMIT,
} from "$lib/server/sync"

import type { RequestHandler } from "./$types"

// the desktop webview posts from its own origin, so the route answers preflight
const ORIGINS = new Set([
  "http://tauri.localhost",
  "https://tauri.localhost",
  "tauri://localhost",
  "http://localhost:1421",
])

function allow(request: Request): Record<string, string> {
  const origin = request.headers.get("origin") ?? ""

  if (!ORIGINS.has(origin)) {
    return {}
  }

  return {
    "access-control-allow-origin": origin,
    "access-control-allow-headers": "authorization, content-type",
    "access-control-allow-methods": "POST, DELETE, OPTIONS",
    "access-control-max-age": "86400",
    vary: "origin",
  }
}

export const OPTIONS: RequestHandler = async ({ request }) =>
  new Response(null, { status: 204, headers: allow(request) })

export const POST: RequestHandler = ({ request }) =>
  answer(async () => {
    const user = await signedIn(request)
    const mine = readPayload(await readJson(request, SYNC_BODY_LIMIT))

    await merge(user.id, mine)

    return json(await everything(user.id, mine.version === 2))
  }, allow(request))

export const DELETE: RequestHandler = ({ request }) =>
  answer(async () => {
    const user = await signedIn(request)

    await forget(user.id)

    return json({ cleared: true })
  }, allow(request))
