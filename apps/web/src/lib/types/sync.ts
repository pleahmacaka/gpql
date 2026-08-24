export interface SyncPreference {
  key: string
  value: string
}

export interface SyncRecent {
  url: string
  kind: string
  label: string
  detail: string
  openedAt: number
}

export interface SyncQuery {
  id: string
  name: string
  sql: string
  target: string
  savedAt: number
}

export interface SyncPayload {
  preferences: SyncPreference[]
  recents: SyncRecent[]
  queries: SyncQuery[]
}
