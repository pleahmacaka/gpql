import { and, eq } from "drizzle-orm"

import { local } from "$lib/db/client"
import { preference, recent, savedQuery, syncTombstone } from "$lib/db/schema"
import * as m from "$lib/paraglide/messages"
import {
  accountToken,
  carriesSecret,
  run,
  stripSecrets,
} from "$lib/session/commands"
import type { SyncPayload, SyncQuery, SyncRecent } from "$lib/types"

const fallback = import.meta.env.DEV
  ? "http://localhost:5173"
  : "https://gpql.dev"

export const site = import.meta.env.VITE_GPQL_SITE ?? fallback

type Buried = (typeof syncTombstone.$inferSelect)["kind"]

const DEVICE_KEYS = ["settled", "asideWidth", "pretty"]
const DEVICE_PREFIXES = ["lsp:", "erd:"]

export function deviceOnly(key: string) {
  return (
    DEVICE_KEYS.includes(key) ||
    DEVICE_PREFIXES.some(prefix => key.startsWith(prefix))
  )
}

const now = () => Math.floor(Date.now() / 1000)

export async function bury(kind: Buried, key: string) {
  if (carriesSecret(key)) {
    return
  }

  const deletedAt = now()

  await local
    .insert(syncTombstone)
    .values({ kind, key, deletedAt })
    .onConflictDoUpdate({
      target: [syncTombstone.kind, syncTombstone.key],
      set: { deletedAt },
    })
}

export async function unbury(kind: Buried, key: string) {
  await local
    .delete(syncTombstone)
    .where(and(eq(syncTombstone.kind, kind), eq(syncTombstone.key, key)))
}

async function refusal(answer: Response) {
  const body: unknown = await answer.json().catch(() => null)

  if (
    body &&
    typeof body === "object" &&
    "message" in body &&
    typeof body.message === "string"
  ) {
    return body.message
  }

  return `${answer.status} ${answer.statusText}`
}

async function ask(method: string, token: string, body?: string) {
  const answer = await fetch(`${site}/api/sync`, {
    method,
    body,
    headers: {
      Authorization: `Bearer ${token}`,
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    signal: AbortSignal.timeout(15_000),
  })

  if (!answer.ok) {
    throw await refusal(answer)
  }

  return answer
}

async function snapshot() {
  const [preferences, recents, queries, graves] = await Promise.all([
    local.select().from(preference),
    local.select().from(recent),
    local.select().from(savedQuery),
    local.select().from(syncTombstone),
  ])

  return { preferences, recents, queries, graves }
}

async function gather(): Promise<SyncPayload> {
  const { preferences, recents, queries, graves } = await snapshot()
  const buried = (kind: Buried) =>
    graves.filter(grave => grave.kind === kind && !carriesSecret(grave.key))

  const liveRecents: SyncRecent[] = recents.map(row => ({
    url: stripSecrets(row.url),
    kind: row.kind,
    label: row.label,
    detail: stripSecrets(row.detail),
    openedAt: row.openedAt,
    deletedAt: null,
  }))

  const deadRecents: SyncRecent[] = buried("recent").map(grave => ({
    url: grave.key,
    kind: "",
    label: "",
    detail: "",
    openedAt: 0,
    deletedAt: grave.deletedAt,
  }))

  const liveQueries: SyncQuery[] = queries.map(row => ({
    id: row.id,
    name: row.name,
    sql: row.sql,
    target: stripSecrets(row.target),
    savedAt: row.savedAt,
    deletedAt: null,
  }))

  const deadQueries: SyncQuery[] = buried("query").map(grave => ({
    id: grave.key,
    name: "",
    sql: "",
    target: "",
    savedAt: 0,
    deletedAt: grave.deletedAt,
  }))

  return {
    version: 2,
    preferences: preferences
      .filter(row => !deviceOnly(row.key) && !carriesSecret(row.key))
      .map(row => ({
        key: row.key,
        value: row.value,
        updatedAt: row.updatedAt,
      })),
    recents: [...liveRecents, ...deadRecents],
    queries: [...liveQueries, ...deadQueries],
  }
}

function newest(...stamps: (number | undefined)[]) {
  const known = stamps.filter(stamp => stamp !== undefined)

  return known.length === 0 ? null : Math.max(...known)
}

async function absorbRecent(
  row: SyncRecent,
  held: Map<string, number>,
  graves: Map<string, number>,
) {
  if (carriesSecret(row.url)) {
    return
  }

  const stamp = Math.max(row.openedAt, row.deletedAt ?? 0)
  const mine = newest(held.get(row.url), graves.get(`recent:${row.url}`))

  if (row.deletedAt !== null) {
    const opened = held.get(row.url)
    const buried = graves.get(`recent:${row.url}`)

    if (opened !== undefined && opened <= stamp) {
      await local.delete(recent).where(eq(recent.url, row.url))
    }

    if (buried !== undefined && buried <= stamp) {
      await unbury("recent", row.url)
    }

    return
  }

  if (mine !== null && stamp < mine) {
    return
  }

  const fields = {
    kind: row.kind,
    label: row.label,
    detail: stripSecrets(row.detail),
    openedAt: row.openedAt,
  }

  await local
    .insert(recent)
    .values({ url: row.url, ...fields })
    .onConflictDoUpdate({ target: recent.url, set: fields })
  await unbury("recent", row.url)
}

async function absorbQuery(
  row: SyncQuery,
  held: Map<string, number>,
  graves: Map<string, number>,
) {
  const stamp = Math.max(row.savedAt, row.deletedAt ?? 0)
  const mine = newest(held.get(row.id), graves.get(`query:${row.id}`))

  if (row.deletedAt !== null) {
    const saved = held.get(row.id)
    const buried = graves.get(`query:${row.id}`)

    if (saved !== undefined && saved <= stamp) {
      await local.delete(savedQuery).where(eq(savedQuery.id, row.id))
    }

    if (buried !== undefined && buried <= stamp) {
      await unbury("query", row.id)
    }

    return
  }

  if (mine !== null && stamp < mine) {
    return
  }

  const fields = {
    name: row.name,
    sql: row.sql,
    target: row.target,
    savedAt: row.savedAt,
  }

  await local
    .insert(savedQuery)
    .values({ id: row.id, ...fields })
    .onConflictDoUpdate({ target: savedQuery.id, set: fields })
  await unbury("query", row.id)
}

async function absorb(theirs: SyncPayload) {
  const { preferences, recents, queries, graves } = await snapshot()

  const prefs = new Map(preferences.map(row => [row.key, row.updatedAt]))
  const opened = new Map(recents.map(row => [row.url, row.openedAt]))
  const saved = new Map(queries.map(row => [row.id, row.savedAt]))
  const buried = new Map(
    graves.map(grave => [`${grave.kind}:${grave.key}`, grave.deletedAt]),
  )

  for (const row of theirs.preferences) {
    const mine = prefs.get(row.key)

    if (
      deviceOnly(row.key) ||
      carriesSecret(row.key) ||
      (mine !== undefined && row.updatedAt < mine)
    ) {
      continue
    }

    await local
      .insert(preference)
      .values({ key: row.key, value: row.value, updatedAt: row.updatedAt })
      .onConflictDoUpdate({
        target: preference.key,
        set: { value: row.value, updatedAt: row.updatedAt },
      })
  }

  for (const row of theirs.recents) {
    await absorbRecent(row, opened, buried)
  }

  for (const row of theirs.queries) {
    await absorbQuery(row, saved, buried)
  }
}

export async function sync(): Promise<string> {
  const token = await run(accountToken())

  if (!token) {
    return m.sync_needs_login()
  }

  const answer = await ask("POST", token, JSON.stringify(await gather()))
  const data = (await answer.json()) as SyncPayload

  await absorb(data)

  return m.sync_done({
    connections: data.recents.filter(row => row.deletedAt === null).length,
    queries: data.queries.filter(row => row.deletedAt === null).length,
  })
}

export async function wipeCloud(): Promise<string> {
  const token = await run(accountToken())

  if (!token) {
    return m.sync_needs_login()
  }

  await ask("DELETE", token)

  return m.sync_cloud_cleared()
}
