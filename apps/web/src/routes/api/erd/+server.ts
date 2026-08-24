import { error, json } from "@sveltejs/kit"

import { auth } from "$lib/server/auth"
import { db } from "$lib/server/db"
import { erdRoom } from "$lib/server/db/sync-schema"

import type { RequestHandler } from "./$types"

export const POST: RequestHandler = async ({ request }) => {
  const session = await auth.api.getSession({ headers: request.headers })

  if (!session) {
    throw error(401, "sign in before sharing a schema")
  }

  const body = (await request.json()) as { name?: string; tables?: unknown }

  if (!Array.isArray(body.tables)) {
    throw error(400, "that schema is not a list of tables")
  }

  const id = crypto.randomUUID().replaceAll("-", "").slice(0, 12)

  await db.insert(erdRoom).values({
    id,
    userId: session.user.id,
    name: body.name ?? "schema",
    tables: JSON.stringify(body.tables),
    createdAt: Math.floor(Date.now() / 1000),
  })

  return json({ id })
}
