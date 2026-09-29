import { betterAuth } from "better-auth"
import { drizzleAdapter } from "better-auth/adapters/drizzle"
import { bearer } from "better-auth/plugins"
import { building, dev } from "$app/environment"

import { db } from "./db"
import * as schema from "./db/schema"

const secret =
  process.env.BETTER_AUTH_SECRETS ||
  process.env.BETTER_AUTH_SECRET ||
  process.env.AUTH_SECRET

if (!secret && !dev && !building) {
  throw new Error(
    "set BETTER_AUTH_SECRET, AUTH_SECRET or BETTER_AUTH_SECRETS before starting",
  )
}

export const auth = betterAuth({
  appName: "gpql",
  baseURL:
    process.env.BETTER_AUTH_URL ?? (dev ? "http://localhost:5173" : undefined),
  database: drizzleAdapter(db, { provider: "pg", schema }),
  socialProviders: {
    github: {
      clientId: process.env.GITHUB_CLIENT_ID ?? "",
      clientSecret: process.env.GITHUB_CLIENT_SECRET ?? "",
    },
  },
  plugins: [bearer()],
})
