export interface SyncPreference {
  key: string
  value: string
  updatedAt: number
}

export interface SyncRecent {
  url: string
  kind: string
  label: string
  detail: string
  openedAt: number
  deletedAt: number | null
}

export interface SyncQuery {
  id: string
  name: string
  sql: string
  target: string
  savedAt: number
  deletedAt: number | null
}

export interface SyncPayload {
  version?: number
  preferences: SyncPreference[]
  recents: SyncRecent[]
  queries: SyncQuery[]
}
