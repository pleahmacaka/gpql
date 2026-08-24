import { sql } from "drizzle-orm"
import { index, integer, sqliteTable, text } from "drizzle-orm/sqlite-core"

export const preference = sqliteTable("preference", {
  key: text("key").primaryKey(),
  value: text("value").notNull(),
})

export const recent = sqliteTable(
  "recent",
  {
    url: text("url").primaryKey(),
    kind: text("kind").notNull(),
    label: text("label").notNull(),
    detail: text("detail").notNull(),
    openedAt: integer("opened_at").notNull().default(sql`(unixepoch())`),
  },
  table => [index("recent_opened_at").on(table.openedAt)],
)

export const savedQuery = sqliteTable("saved_query", {
  id: text("id").primaryKey(),
  name: text("name").notNull(),
  sql: text("sql").notNull(),
  target: text("target").notNull().default(""),
  savedAt: integer("saved_at").notNull().default(sql`(unixepoch())`),
})

export type Recent = typeof recent.$inferSelect
export type SavedQuery = typeof savedQuery.$inferSelect
