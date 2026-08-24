import type { preference, recent, savedQuery } from "$lib/db/schema"

export interface SyncPayload {
  preferences: (typeof preference.$inferSelect)[]
  recents: (typeof recent.$inferSelect)[]
  queries: (typeof savedQuery.$inferSelect)[]
}
