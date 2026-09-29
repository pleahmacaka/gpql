import { json } from "@sveltejs/kit"
import { and, desc, eq } from "drizzle-orm"

import { db } from "$lib/server/db"
import { erdRoom } from "$lib/server/db/sync-schema"
import {
  ROOM_BODY_LIMIT,
  ROOMS_PER_USER,
  readId,
  readName,
  readTables,
} from "$lib/server/erd"
import { answer, isRecord, Refusal, readJson, signedIn } from "$lib/server/http"
import type { ErdRoomSummary } from "$lib/types"

import type { RequestHandler } from "./$types"

async function readBody(request: Request) {
  const body = await readJson(request, ROOM_BODY_LIMIT)

  if (!isRecord(body)) {
    throw new Refusal(400, "the body must be an object")
  }

  return body
}

const owned = (id: string, userId: string) =>
  and(eq(erdRoom.id, id), eq(erdRoom.userId, userId))

const missing = () => new Refusal(404, "no schema of yours behind that link")

export const GET: RequestHandler = ({ request }) =>
  answer(async () => {
    const user = await signedIn(request)

    const rooms: ErdRoomSummary[] = (
      await db
        .select({
          id: erdRoom.id,
          name: erdRoom.name,
          open: erdRoom.open,
          createdAt: erdRoom.createdAt,
        })
        .from(erdRoom)
        .where(eq(erdRoom.userId, user.id))
        .orderBy(desc(erdRoom.createdAt))
    ).map(room => ({ ...room, open: room.open === 1 }))

    return json({ rooms })
  })

export const POST: RequestHandler = ({ request }) =>
  answer(async () => {
    const user = await signedIn(request)
    const body = await readBody(request)
    const name = readName(body.name)
    const tables = readTables(body.tables)
    const open = body.open === true

    if (
      (await db.$count(erdRoom, eq(erdRoom.userId, user.id))) >= ROOMS_PER_USER
    ) {
      throw new Refusal(
        409,
        `you already share ${ROOMS_PER_USER} schemas, close one first`,
      )
    }

    const id = crypto.randomUUID().replaceAll("-", "").slice(0, 12)

    await db.insert(erdRoom).values({
      id,
      userId: user.id,
      name,
      tables,
      open: open ? 1 : 0,
      createdAt: Math.floor(Date.now() / 1000),
    })

    return json({ id, open })
  })

export const PUT: RequestHandler = ({ request }) =>
  answer(async () => {
    const user = await signedIn(request)
    const body = await readBody(request)
    const id = readId(body.id)

    const [room] = await db
      .update(erdRoom)
      .set({ name: readName(body.name), tables: readTables(body.tables) })
      .where(owned(id, user.id))
      .returning({ id: erdRoom.id, open: erdRoom.open })

    if (!room) {
      throw missing()
    }

    return json({ id: room.id, open: room.open === 1 })
  })

export const PATCH: RequestHandler = ({ request }) =>
  answer(async () => {
    const user = await signedIn(request)
    const body = await readBody(request)
    const id = readId(body.id)

    if (typeof body.open !== "boolean") {
      throw new Refusal(400, "open must be true or false")
    }

    const [room] = await db
      .update(erdRoom)
      .set({ open: body.open ? 1 : 0 })
      .where(owned(id, user.id))
      .returning({ id: erdRoom.id, open: erdRoom.open })

    if (!room) {
      throw missing()
    }

    return json({ id: room.id, open: room.open === 1 })
  })

export const DELETE: RequestHandler = ({ request }) =>
  answer(async () => {
    const user = await signedIn(request)
    const body = await readBody(request)
    const id = readId(body.id)

    const gone = await db
      .delete(erdRoom)
      .where(owned(id, user.id))
      .returning({ id: erdRoom.id })

    if (gone.length === 0) {
      throw missing()
    }

    return json({ closed: true })
  })
