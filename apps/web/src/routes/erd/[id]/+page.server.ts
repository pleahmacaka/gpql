import { error, redirect } from "@sveltejs/kit"
import { eq } from "drizzle-orm"

import { auth } from "$lib/server/auth"
import { db } from "$lib/server/db"
import { erdRoom } from "$lib/server/db/sync-schema"

import type { PageServerLoad } from "./$types"

export const load: PageServerLoad = async ({ request, params }) => {
  const session = await auth.api.getSession({ headers: request.headers })

  if (!session) {
    redirect(303, "/account")
  }

  const [room] = await db
    .select()
    .from(erdRoom)
    .where(eq(erdRoom.id, params.id))
    .limit(1)

  if (!room) {
    throw error(404, "no schema behind that link")
  }

  return {
    room: { id: room.id, name: room.name, tables: JSON.parse(room.tables) },
    who: session.user.name,
  }
}
