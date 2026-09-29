import { and, eq, inArray, type SQL, sql } from "drizzle-orm"
import type { PgColumn } from "drizzle-orm/pg-core"

import type {
  SyncPayload,
  SyncPreference,
  SyncQuery,
  SyncRecent,
} from "$lib/types"

import { db } from "./db"
import { syncPreference, syncQuery, syncRecent } from "./db/sync-schema"
import { isRecord, Refusal } from "./http"
import { carriesSecret, stripSecrets } from "./secrets"

// bytes; past Vercel's 4.5 MB cap the platform answers 413 without CORS
export const SYNC_BODY_LIMIT = 4_000_000

const MAX_ROWS = 5000
const MAX_STAMP = 2_147_483_647

type Field<T> = (value: unknown, path: string) => T

const text =
  (max: number): Field<string> =>
  (value, path) => {
    if (typeof value !== "string") {
      throw new Refusal(400, `${path} must be a string`)
    }

    if (value.length > max) {
      throw new Refusal(400, `${path} is longer than ${max} characters`)
    }

    return value
  }

const stamp: Field<number> = (value, path) => {
  if (typeof value !== "number" || !Number.isInteger(value)) {
    throw new Refusal(400, `${path} must be a whole number of seconds`)
  }

  if (value < 0 || value > MAX_STAMP) {
    throw new Refusal(400, `${path} is out of range`)
  }

  return value
}

const stampOr =
  <T>(missing: T): Field<number | T> =>
  (value, path) =>
    value === undefined || value === null ? missing : stamp(value, path)

function rows<T>(
  body: Record<string, unknown>,
  name: string,
  read: (row: Record<string, unknown>, path: string) => T,
) {
  const list = body[name]

  if (!Array.isArray(list)) {
    throw new Refusal(400, `${name} must be a list`)
  }

  if (list.length > MAX_ROWS) {
    throw new Refusal(400, `${name} holds more than ${MAX_ROWS} rows`)
  }

  return list.map((row, index) => {
    const path = `${name}[${index}]`

    if (!isRecord(row)) {
      throw new Refusal(400, `${path} must be an object`)
    }

    return read(row, path)
  })
}

export function readPayload(body: unknown): SyncPayload {
  if (!isRecord(body)) {
    throw new Refusal(400, "the body must be an object")
  }

  // 0.5.0 sends no version or stamps, and its setting change must still win
  const current = body.version === 2
  const arrived = Math.floor(Date.now() / 1000)

  return {
    version: current ? 2 : 1,
    preferences: rows(body, "preferences", (row, path) => ({
      key: text(256)(row.key, `${path}.key`),
      value: text(262_144)(row.value, `${path}.value`),
      updatedAt: current
        ? stampOr(0)(row.updatedAt, `${path}.updatedAt`)
        : arrived,
    })),
    recents: rows(body, "recents", (row, path) => ({
      url: text(4096)(row.url, `${path}.url`),
      kind: text(64)(row.kind, `${path}.kind`),
      label: text(512)(row.label, `${path}.label`),
      detail: text(4096)(row.detail, `${path}.detail`),
      openedAt: stamp(row.openedAt, `${path}.openedAt`),
      deletedAt: stampOr(null)(row.deletedAt, `${path}.deletedAt`),
    })),
    queries: rows(body, "queries", (row, path) => ({
      id: text(256)(row.id, `${path}.id`),
      name: text(512)(row.name, `${path}.name`),
      sql: text(262_144)(row.sql, `${path}.sql`),
      target: text(4096)(row.target ?? "", `${path}.target`),
      savedAt: stamp(row.savedAt, `${path}.savedAt`),
      deletedAt: stampOr(null)(row.deletedAt, `${path}.deletedAt`),
    })),
  }
}

function newest<T>(
  list: T[],
  key: (row: T) => string,
  weight: (row: T) => number,
) {
  const kept = new Map<string, T>()

  for (const row of list) {
    const held = kept.get(key(row))

    if (!held || weight(row) >= weight(held)) {
      kept.set(key(row), row)
    }
  }

  return [...kept.values()]
}

const excluded = (column: PgColumn) =>
  sql`excluded.${sql.identifier(column.name)}`

const weight = (at: SQL | PgColumn, gone?: SQL | PgColumn) =>
  gone ? sql`greatest(${at}, coalesce(${gone}, 0))` : sql`${at}`

const preferenceWeight = (row: SyncPreference) => row.updatedAt
const recentWeight = (row: SyncRecent) =>
  Math.max(row.openedAt, row.deletedAt ?? 0)
const queryWeight = (row: SyncQuery) =>
  Math.max(row.savedAt, row.deletedAt ?? 0)

export async function merge(userId: string, mine: SyncPayload) {
  const preferences = newest(
    mine.preferences.filter(r => !carriesSecret(r.key)),
    r => r.key,
    preferenceWeight,
  )
  const recents = newest(
    mine.recents
      .filter(r => !carriesSecret(r.url))
      .map(r => ({ ...r, detail: stripSecrets(r.detail) })),
    r => r.url,
    recentWeight,
  )
  const queries = newest(
    mine.queries.map(r => ({ ...r, target: stripSecrets(r.target) })),
    r => r.id,
    queryWeight,
  )

  await db.transaction(async tx => {
    if (preferences.length > 0) {
      await tx
        .insert(syncPreference)
        .values(preferences.map(row => ({ ...row, userId })))
        .onConflictDoUpdate({
          target: [syncPreference.userId, syncPreference.key],
          set: {
            value: excluded(syncPreference.value),
            updatedAt: excluded(syncPreference.updatedAt),
          },
          setWhere: sql`${excluded(syncPreference.updatedAt)} >= ${syncPreference.updatedAt}`,
        })
    }

    if (recents.length > 0) {
      await tx
        .insert(syncRecent)
        .values(recents.map(row => ({ ...row, userId })))
        .onConflictDoUpdate({
          target: [syncRecent.userId, syncRecent.url],
          set: {
            kind: excluded(syncRecent.kind),
            label: excluded(syncRecent.label),
            detail: excluded(syncRecent.detail),
            openedAt: excluded(syncRecent.openedAt),
            deletedAt: excluded(syncRecent.deletedAt),
          },
          setWhere: sql`${weight(
            excluded(syncRecent.openedAt),
            excluded(syncRecent.deletedAt),
          )} >= ${weight(syncRecent.openedAt, syncRecent.deletedAt)}`,
        })
    }

    if (queries.length > 0) {
      await tx
        .insert(syncQuery)
        .values(queries.map(row => ({ ...row, userId })))
        .onConflictDoUpdate({
          target: [syncQuery.userId, syncQuery.id],
          set: {
            name: excluded(syncQuery.name),
            sql: excluded(syncQuery.sql),
            target: excluded(syncQuery.target),
            savedAt: excluded(syncQuery.savedAt),
            deletedAt: excluded(syncQuery.deletedAt),
          },
          setWhere: sql`${weight(
            excluded(syncQuery.savedAt),
            excluded(syncQuery.deletedAt),
          )} >= ${weight(syncQuery.savedAt, syncQuery.deletedAt)}`,
        })
    }
  })
}

async function purge(
  userId: string,
  preferences: SyncPreference[],
  recents: SyncRecent[],
  queries: SyncQuery[],
) {
  const keys = preferences.map(r => r.key).filter(carriesSecret)
  const urls = recents.map(r => r.url).filter(carriesSecret)
  const details = recents.filter(
    r => !carriesSecret(r.url) && carriesSecret(r.detail),
  )
  const targets = queries.filter(r => carriesSecret(r.target))

  if (keys.length + urls.length + details.length + targets.length === 0) {
    return
  }

  await db.transaction(async tx => {
    if (keys.length > 0) {
      await tx
        .delete(syncPreference)
        .where(
          and(
            eq(syncPreference.userId, userId),
            inArray(syncPreference.key, keys),
          ),
        )
    }

    if (urls.length > 0) {
      await tx
        .delete(syncRecent)
        .where(
          and(eq(syncRecent.userId, userId), inArray(syncRecent.url, urls)),
        )
    }

    for (const row of details) {
      await tx
        .update(syncRecent)
        .set({ detail: stripSecrets(row.detail) })
        .where(and(eq(syncRecent.userId, userId), eq(syncRecent.url, row.url)))
    }

    for (const row of targets) {
      await tx
        .update(syncQuery)
        .set({ target: stripSecrets(row.target) })
        .where(and(eq(syncQuery.userId, userId), eq(syncQuery.id, row.id)))
    }
  })
}

export async function everything(
  userId: string,
  tombstones: boolean,
): Promise<SyncPayload> {
  const [preferences, recents, queries] = await Promise.all([
    db
      .select({
        key: syncPreference.key,
        value: syncPreference.value,
        updatedAt: syncPreference.updatedAt,
      })
      .from(syncPreference)
      .where(eq(syncPreference.userId, userId)),
    db
      .select({
        url: syncRecent.url,
        kind: syncRecent.kind,
        label: syncRecent.label,
        detail: syncRecent.detail,
        openedAt: syncRecent.openedAt,
        deletedAt: syncRecent.deletedAt,
      })
      .from(syncRecent)
      .where(eq(syncRecent.userId, userId)),
    db
      .select({
        id: syncQuery.id,
        name: syncQuery.name,
        sql: syncQuery.sql,
        target: syncQuery.target,
        savedAt: syncQuery.savedAt,
        deletedAt: syncQuery.deletedAt,
      })
      .from(syncQuery)
      .where(eq(syncQuery.userId, userId)),
  ])

  await purge(userId, preferences, recents, queries)

  const kept = <T extends { deletedAt: number | null }>(list: T[]) =>
    tombstones ? list : list.filter(r => r.deletedAt === null)

  return {
    preferences: preferences.filter(r => !carriesSecret(r.key)),
    recents: kept(recents)
      .filter(r => !carriesSecret(r.url))
      .map(r => ({ ...r, detail: stripSecrets(r.detail) })),
    queries: kept(queries).map(r => ({
      ...r,
      target: stripSecrets(r.target),
    })),
  }
}

export async function forget(userId: string) {
  await db.transaction(async tx => {
    await tx.delete(syncPreference).where(eq(syncPreference.userId, userId))
    await tx.delete(syncRecent).where(eq(syncRecent.userId, userId))
    await tx.delete(syncQuery).where(eq(syncQuery.userId, userId))
  })
}
