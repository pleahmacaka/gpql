import { integer, primaryKey, text } from "drizzle-orm/pg-core"

import { gpql } from "./area"

import { user } from "./auth-schema"

const owner = () =>
  text("user_id")
    .notNull()
    .references(() => user.id, { onDelete: "cascade" })

export const syncPreference = gpql.table(
  "sync_preference",
  {
    userId: owner(),
    key: text("key").notNull(),
    value: text("value").notNull(),
  },
  table => [primaryKey({ columns: [table.userId, table.key] })],
)

export const syncRecent = gpql.table(
  "sync_recent",
  {
    userId: owner(),
    url: text("url").notNull(),
    kind: text("kind").notNull(),
    label: text("label").notNull(),
    detail: text("detail").notNull(),
    openedAt: integer("opened_at").notNull(),
  },
  table => [primaryKey({ columns: [table.userId, table.url] })],
)

export const syncQuery = gpql.table("sync_query", {
  id: text("id").primaryKey(),
  userId: owner(),
  name: text("name").notNull(),
  sql: text("sql").notNull(),
  target: text("target").notNull().default(""),
  savedAt: integer("saved_at").notNull(),
})

export const erdRoom = gpql.table("erd_room", {
  id: text("id").primaryKey(),
  userId: owner(),
  name: text("name").notNull(),
  tables: text("tables").notNull(),
  createdAt: integer("created_at").notNull(),
})
