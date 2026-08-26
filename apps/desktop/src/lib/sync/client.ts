import axios from "axios"
import { desc } from "drizzle-orm"

import { local } from "$lib/db/client"
import { preference, recent, savedQuery } from "$lib/db/schema"
import { accountToken, run } from "$lib/session/commands"
import type { SyncPayload } from "$lib/types"

const fallback = import.meta.env.DEV
  ? "http://localhost:5173"
  : "https://gpql.app"

export const site = import.meta.env.VITE_GPQL_SITE ?? fallback

export async function sync(): Promise<string> {
  const token = await run(accountToken())

  if (!token) {
    return "sign in first"
  }

  const mine: SyncPayload = {
    preferences: await local.select().from(preference),
    recents: await local.select().from(recent).orderBy(desc(recent.openedAt)),
    queries: await local.select().from(savedQuery),
  }

  const { data } = await axios.post<SyncPayload>(`${site}/api/sync`, mine, {
    headers: { Authorization: `Bearer ${token}` },
    timeout: 15_000,
  })

  await absorb(data)

  return `synced ${data.recents.length} connections and ${data.queries.length} queries`
}

async function absorb(theirs: SyncPayload) {
  for (const row of theirs.preferences) {
    await local
      .insert(preference)
      .values(row)
      .onConflictDoUpdate({
        target: preference.key,
        set: { value: row.value },
      })
  }

  for (const row of theirs.recents) {
    await local
      .insert(recent)
      .values(row)
      .onConflictDoUpdate({
        target: recent.url,
        set: { openedAt: row.openedAt, label: row.label, detail: row.detail },
      })
  }

  for (const row of theirs.queries) {
    await local
      .insert(savedQuery)
      .values(row)
      .onConflictDoUpdate({
        target: savedQuery.id,
        set: { name: row.name, sql: row.sql, savedAt: row.savedAt },
      })
  }
}

export async function wipeCloud(): Promise<string> {
  const token = await run(accountToken())

  if (!token) {
    return "sign in first"
  }

  await axios.delete(`${site}/api/sync`, {
    headers: { Authorization: `Bearer ${token}` },
    timeout: 15_000,
  })

  return "cloud data cleared"
}
