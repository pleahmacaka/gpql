import { eq } from "drizzle-orm"
import * as Y from "yjs"

import { db } from "./db"
import { erdRoom } from "./db/sync-schema"
import { isRecord, Refusal } from "./http"
import { canSee } from "./sharing"

export const ROOM_BODY_LIMIT = 2_500_000
export const ROOMS_PER_USER = 100

const MAX_NAME = 200
const MAX_TABLES_JSON = 2_000_000

// the map shareLayout in packages/ui writes; compaction copies nothing else
const POSITIONS = "positions"

// mergeUpdates keeps every overwritten position forever; a doc collects them
export function fold(held: Uint8Array, update: Uint8Array) {
  const doc = new Y.Doc()

  try {
    Y.applyUpdate(doc, held)
    Y.applyUpdate(doc, update)

    return Buffer.from(Y.encodeStateAsUpdate(doc))
  } catch {
    throw new Refusal(400, "update is not a Yjs update")
  } finally {
    doc.destroy()
  }
}

export function compact(state: Uint8Array) {
  const held = new Y.Doc()
  const fresh = new Y.Doc()

  try {
    Y.applyUpdate(held, state)

    const positions = fresh.getMap<unknown>(POSITIONS)

    fresh.transact(() => {
      for (const [key, spot] of held.getMap<unknown>(POSITIONS).entries()) {
        positions.set(key, spot)
      }
    })

    return Buffer.from(Y.encodeStateAsUpdate(fresh))
  } finally {
    held.destroy()
    fresh.destroy()
  }
}

export function readName(value: unknown) {
  if (value === undefined) {
    return "schema"
  }

  if (typeof value !== "string" || value.length > MAX_NAME) {
    throw new Refusal(
      400,
      `name must be a string of at most ${MAX_NAME} characters`,
    )
  }

  return value.trim() || "schema"
}

function isTable(value: unknown) {
  return (
    isRecord(value) &&
    typeof value.name === "string" &&
    Array.isArray(value.columns) &&
    value.columns.every(
      column => isRecord(column) && typeof column.name === "string",
    )
  )
}

export function readTables(value: unknown) {
  if (!Array.isArray(value) || !value.every(isTable)) {
    throw new Refusal(400, "tables must be a list of tables with named columns")
  }

  const tables = JSON.stringify(value)

  if (tables.length > MAX_TABLES_JSON) {
    throw new Refusal(413, "that schema is too large to share")
  }

  return tables
}

export function readId(value: unknown) {
  if (typeof value !== "string" || value.length === 0 || value.length > 64) {
    throw new Refusal(400, "id must name a room")
  }

  return value
}

export async function visibleRoom(id: string, viewerId: string | null) {
  const [room] = await db
    .select({
      userId: erdRoom.userId,
      open: erdRoom.open,
      version: erdRoom.version,
      epoch: erdRoom.epoch,
    })
    .from(erdRoom)
    .where(eq(erdRoom.id, id))
    .limit(1)

  if (!room) {
    throw new Refusal(404, "no schema behind that link")
  }

  const verdict = canSee(room, viewerId)

  if (verdict === "sign-in") {
    throw new Refusal(401, "sign in to see that schema")
  }

  if (verdict === "hide") {
    throw new Refusal(403, "that schema is not shared with you")
  }

  return room
}
