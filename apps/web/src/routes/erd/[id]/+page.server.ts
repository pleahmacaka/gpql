import { error, redirect } from "@sveltejs/kit"
import { eq } from "drizzle-orm"

import { auth } from "$lib/server/auth"
import { db } from "$lib/server/db"
import { erdRoom } from "$lib/server/db/sync-schema"
import { canSee } from "$lib/server/sharing"

import type { PageServerLoad } from "./$types"

export const load: PageServerLoad = async ({ request, params }) => {
  const session = await auth.api.getSession({ headers: request.headers })

  const [room] = await db
    .select()
    .from(erdRoom)
    .where(eq(erdRoom.id, params.id))
    .limit(1)

  if (!room) {
    throw error(404, "no schema behind that link")
  }

  const verdict = canSee(room, session?.user.id ?? null)

  if (verdict === "sign-in") {
    redirect(303, "/account")
  }

  if (verdict === "hide") {
    throw error(403, "that schema is not shared with you")
  }

  return {
    room: {
      id: room.id,
      name: room.name,
      tables: JSON.parse(room.tables),
      open: room.open === 1,
    },
    who: session?.user.name ?? "guest",
  }
}
