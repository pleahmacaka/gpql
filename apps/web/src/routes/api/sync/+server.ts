import { json } from "@sveltejs/kit"
import { eq } from "drizzle-orm"
import { auth } from "$lib/server/auth"
import { db } from "$lib/server/db"
import {
  syncPreference,
  syncQuery,
  syncRecent,
} from "$lib/server/db/sync-schema"
import type { SyncPayload } from "$lib/types"

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

export const POST: RequestHandler = async ({ request }) => {
  const session = await auth.api.getSession({ headers: request.headers })

  if (!session) {
    return json(
      { message: "sign in first" },
      { status: 401, headers: allow(request) },
    )
  }

  const userId = session.user.id
  const mine = (await request.json()) as SyncPayload

  await db.transaction(async tx => {
    for (const row of mine.preferences) {
      await tx
        .insert(syncPreference)
        .values({ ...row, userId })
        .onConflictDoUpdate({
          target: [syncPreference.userId, syncPreference.key],
          set: { value: row.value },
        })
    }

    for (const row of mine.recents) {
      await tx
        .insert(syncRecent)
        .values({ ...row, userId })
        .onConflictDoUpdate({
          target: [syncRecent.userId, syncRecent.url],
          set: { label: row.label, detail: row.detail, openedAt: row.openedAt },
        })
    }

    for (const row of mine.queries) {
      await tx
        .insert(syncQuery)
        .values({ ...row, userId })
        .onConflictDoUpdate({
          target: syncQuery.id,
          set: { name: row.name, sql: row.sql, savedAt: row.savedAt },
          setWhere: eq(syncQuery.userId, userId),
        })
    }
  })

  return json(
    {
      preferences: await db
        .select({ key: syncPreference.key, value: syncPreference.value })
        .from(syncPreference)
        .where(eq(syncPreference.userId, userId)),
      recents: await db
        .select({
          url: syncRecent.url,
          kind: syncRecent.kind,
          label: syncRecent.label,
          detail: syncRecent.detail,
          openedAt: syncRecent.openedAt,
        })
        .from(syncRecent)
        .where(eq(syncRecent.userId, userId)),
      queries: await db
        .select({
          id: syncQuery.id,
          name: syncQuery.name,
          sql: syncQuery.sql,
          target: syncQuery.target,
          savedAt: syncQuery.savedAt,
        })
        .from(syncQuery)
        .where(eq(syncQuery.userId, userId)),
    },
    { headers: allow(request) },
  )
}

export const DELETE: RequestHandler = async ({ request }) => {
  const session = await auth.api.getSession({ headers: request.headers })

  if (!session) {
    return json(
      { message: "sign in first" },
      { status: 401, headers: allow(request) },
    )
  }

  const userId = session.user.id

  await db.transaction(async tx => {
    await tx.delete(syncPreference).where(eq(syncPreference.userId, userId))
    await tx.delete(syncRecent).where(eq(syncRecent.userId, userId))
    await tx.delete(syncQuery).where(eq(syncQuery.userId, userId))
  })

  return json({ cleared: true }, { headers: allow(request) })
}
