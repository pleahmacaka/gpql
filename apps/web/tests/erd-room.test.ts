import { expect, test } from "bun:test"
import { and, eq } from "drizzle-orm"
import { drizzle } from "drizzle-orm/postgres-js"
import postgres from "postgres"
import { user } from "../src/lib/server/db/auth-schema"
import { erdRoom } from "../src/lib/server/db/sync-schema"

const url = process.env.GPQL_TEST_PG_URL

const live = url ? test : test.skip

function open() {
  return drizzle(postgres(url as string, { prepare: false }))
}

async function seed(db: ReturnType<typeof open>, id: string) {
  await db.delete(erdRoom).where(eq(erdRoom.id, id))

  await db
    .insert(user)
    .values([
      {
        id: `owner-${id}`,
        name: "owner",
        email: `owner-${id}@x`,
        emailVerified: false,
        createdAt: new Date(),
        updatedAt: new Date(),
      },
      {
        id: `other-${id}`,
        name: "other",
        email: `other-${id}@x`,
        emailVerified: false,
        createdAt: new Date(),
        updatedAt: new Date(),
      },
    ])
    .onConflictDoNothing()

  await db.insert(erdRoom).values({
    id,
    userId: `owner-${id}`,
    name: "schema",
    tables: "[]",
    open: 0,
    createdAt: 1,
  })
}

live("a published room starts private", async () => {
  const db = open()

  await seed(db, "room-private")

  const [room] = await db
    .select()
    .from(erdRoom)
    .where(eq(erdRoom.id, "room-private"))

  expect(room.open).toBe(0)
})

live("the owner can open a room to anyone with the link", async () => {
  const db = open()

  await seed(db, "room-flip")

  const changed = await db
    .update(erdRoom)
    .set({ open: 1 })
    .where(
      and(eq(erdRoom.id, "room-flip"), eq(erdRoom.userId, "owner-room-flip")),
    )
    .returning({ open: erdRoom.open })

  expect(changed).toHaveLength(1)
  expect(changed[0].open).toBe(1)
})

live("another account cannot open someone else's room", async () => {
  const db = open()

  await seed(db, "room-guard")

  const changed = await db
    .update(erdRoom)
    .set({ open: 1 })
    .where(
      and(eq(erdRoom.id, "room-guard"), eq(erdRoom.userId, "other-room-guard")),
    )
    .returning({ open: erdRoom.open })

  expect(changed).toHaveLength(0)

  const [room] = await db
    .select()
    .from(erdRoom)
    .where(eq(erdRoom.id, "room-guard"))

  expect(room.open).toBe(0)
})
