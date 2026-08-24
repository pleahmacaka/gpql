import { error, json } from "@sveltejs/kit"
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

export const POST: RequestHandler = async ({ request }) => {
  const session = await auth.api.getSession({ headers: request.headers })

  if (!session) {
    throw error(401, "sign in first, then put the token in ~/.gpql-account")
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
        })
    }
  })

  return json({
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
  })
}
