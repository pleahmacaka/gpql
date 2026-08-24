import { drizzle } from "drizzle-orm/postgres-js"
import postgres from "postgres"

import * as schema from "./schema"

if (!process.env.DATABASE_URL) {
  process.loadEnvFile?.()
}

const url = process.env.DATABASE_URL

if (!url) {
  throw new Error("DATABASE_URL is not set")
}

export const db = drizzle(postgres(url), { schema })
