import { sql } from "drizzle-orm"
import {
  index,
  integer,
  primaryKey,
  sqliteTable,
  text,
} from "drizzle-orm/sqlite-core"

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
    alias: text("alias"),
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

export const chatLog = sqliteTable("chat_log", {
  id: text("id").primaryKey(),
  title: text("title").notNull().default(""),
  turns: text("turns").notNull(),
  savedAt: integer("saved_at").notNull().default(sql`(unixepoch())`),
})

export const favorite = sqliteTable(
  "favorite",
  {
    target: text("target").notNull(),
    table: text("table").notNull(),
  },
  entry => [primaryKey({ columns: [entry.target, entry.table] })],
)

export const queryRun = sqliteTable(
  "query_run",
  {
    id: text("id").primaryKey(),
    sql: text("sql").notNull(),
    target: text("target").notNull().default(""),
    ok: integer("ok", { mode: "boolean" }).notNull().default(true),
    millis: integer("millis").notNull().default(0),
    ranAt: integer("ran_at").notNull().default(sql`(unixepoch())`),
  },
  table => [index("query_run_ran_at").on(table.ranAt)],
)
