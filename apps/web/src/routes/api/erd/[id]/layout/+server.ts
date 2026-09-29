import { json } from "@sveltejs/kit"
import { eq, sql } from "drizzle-orm"
import * as Y from "yjs"

import { db } from "$lib/server/db"
import { erdRoom } from "$lib/server/db/sync-schema"
import { compact, fold, visibleRoom } from "$lib/server/erd"
import { answer, isRecord, Refusal, readJson, viewer } from "$lib/server/http"
import type { LayoutState } from "$lib/types"

import type { RequestHandler } from "./$types"

const MAX_UPDATE = 256 * 1024
const MAX_LAYOUT = 2 * 1024 * 1024
const COMPACT_AT = 256 * 1024
const EMPTY = Buffer.from(Y.encodeStateAsUpdate(new Y.Doc()))

const fresh = { "cache-control": "no-store" }

function decode(value: unknown) {
  if (typeof value !== "string") {
    throw new Refusal(400, "update must be a base64 string")
  }

  const bytes = Buffer.from(value, "base64")

  if (bytes.toString("base64") !== value) {
    throw new Refusal(400, "update is not valid base64")
  }

  if (bytes.length > MAX_UPDATE) {
    throw new Refusal(413, "that layout update is too large")
  }

  return bytes
}

function readEpoch(value: unknown) {
  if (value === undefined) {
    return null
  }

  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    throw new Refusal(400, "epoch must be a whole number")
  }

  return value
}

export const GET: RequestHandler = ({ request, params, url }) =>
  answer(async () => {
    const user = await viewer(request)
    const room = await visibleRoom(params.id, user?.id ?? null)
    const known = Number(url.searchParams.get("version") ?? -1)
    const knownEpoch = url.searchParams.get("epoch")

    const idle: LayoutState = {
      version: room.version,
      epoch: room.epoch,
      state: null,
    }

    if (
      known === room.version &&
      (knownEpoch === null || Number(knownEpoch) === room.epoch)
    ) {
      return json(idle)
    }

    const [held] = await db
      .select({
        version: erdRoom.version,
        epoch: erdRoom.epoch,
        state: erdRoom.layout,
      })
      .from(erdRoom)
      .where(eq(erdRoom.id, params.id))

    return json(held ?? idle)
  }, fresh)

export const POST: RequestHandler = ({ request, params }) =>
  answer(async () => {
    const user = await viewer(request)

    await visibleRoom(params.id, user?.id ?? null)

    const body = await readJson(request, MAX_UPDATE * 2)

    if (!isRecord(body)) {
      throw new Refusal(400, "the body must be an object")
    }

    const update = decode(body.update)
    const epoch = readEpoch(body.epoch)

    const saved = await db.transaction(async tx => {
      const [room] = await tx
        .select({
          layout: erdRoom.layout,
          version: erdRoom.version,
          epoch: erdRoom.epoch,
        })
        .from(erdRoom)
        .where(eq(erdRoom.id, params.id))
        .for("update")

      if (!room) {
        throw new Refusal(404, "no schema behind that link")
      }

      const current = { version: room.version, epoch: room.epoch }

      if (epoch !== null && epoch !== room.epoch) {
        return current
      }

      const held = room.layout ? Buffer.from(room.layout, "base64") : EMPTY
      const merged = fold(held, update)

      if (merged.equals(held)) {
        return current
      }

      const shrunk = merged.length > COMPACT_AT ? compact(merged) : null
      // a layout that is mostly live positions would compact, and restart every board, on each move
      const compacting = shrunk !== null && shrunk.length * 2 <= merged.length
      const kept = compacting ? shrunk : merged

      if (kept.length > MAX_LAYOUT) {
        throw new Refusal(413, "this layout has grown too large to keep")
      }

      const [stored] = await tx
        .update(erdRoom)
        .set({
          layout: kept.toString("base64"),
          version: sql`${erdRoom.version} + 1`,
          epoch: compacting ? sql`${erdRoom.epoch} + 1` : room.epoch,
        })
        .where(eq(erdRoom.id, params.id))
        .returning({ version: erdRoom.version, epoch: erdRoom.epoch })

      return stored
    })

    return json(saved)
  }, fresh)
