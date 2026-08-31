import { error, json } from "@sveltejs/kit"
import { and, eq } from "drizzle-orm"

import { auth } from "$lib/server/auth"
import { db } from "$lib/server/db"
import { erdRoom } from "$lib/server/db/sync-schema"

import type { RequestHandler } from "./$types"

export const POST: RequestHandler = async ({ request }) => {
  const session = await auth.api.getSession({ headers: request.headers })

  if (!session) {
    throw error(401, "sign in before sharing a schema")
  }

  const body = (await request.json()) as {
    name?: string
    tables?: unknown
    open?: boolean
  }

  if (!Array.isArray(body.tables)) {
    throw error(400, "that schema is not a list of tables")
  }

  const id = crypto.randomUUID().replaceAll("-", "").slice(0, 12)

  await db.insert(erdRoom).values({
    id,
    userId: session.user.id,
    name: body.name ?? "schema",
    tables: JSON.stringify(body.tables),
    open: body.open ? 1 : 0,
    createdAt: Math.floor(Date.now() / 1000),
  })

  return json({ id, open: body.open === true })
}

export const PATCH: RequestHandler = async ({ request }) => {
  const session = await auth.api.getSession({ headers: request.headers })

  if (!session) {
    throw error(401, "sign in before changing a shared schema")
  }

  const body = (await request.json()) as { id?: string; open?: boolean }

  if (!body.id) {
    throw error(400, "no room to change")
  }

  const changed = await db
    .update(erdRoom)
    .set({ open: body.open ? 1 : 0 })
    .where(and(eq(erdRoom.id, body.id), eq(erdRoom.userId, session.user.id)))
    .returning({ id: erdRoom.id, open: erdRoom.open })

  const room = changed[0]

  if (!room) {
    throw error(404, "no schema of yours behind that link")
  }

  return json({ id: room.id, open: room.open === 1 })
}
