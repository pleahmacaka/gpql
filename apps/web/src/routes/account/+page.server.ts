import { redirect } from "@sveltejs/kit"

import { auth } from "$lib/server/auth"
import type { DesktopHandoff } from "$lib/types"

import type { PageServerLoad } from "./$types"

const STATE = /^[0-9a-f]{32}$/

function loopback(url: URL): DesktopHandoff | null {
  const port = Number(url.searchParams.get("port"))
  const state = url.searchParams.get("state") ?? ""

  if (!Number.isInteger(port) || port < 1024 || port > 65535) {
    return null
  }

  if (!STATE.test(state)) {
    return null
  }

  return { port, state }
}

export const load: PageServerLoad = async ({ request, url }) => {
  const session = await auth.api.getSession({ headers: request.headers })
  const handoff = loopback(url)

  if (session && handoff) {
    const target = new URL(`http://127.0.0.1:${handoff.port}/`)

    target.searchParams.set("token", session.session.token)
    target.searchParams.set("state", handoff.state)

    redirect(303, target.toString())
  }

  return {
    account: session
      ? { name: session.user.name, email: session.user.email }
      : null,
    handoff: handoff ? `?port=${handoff.port}&state=${handoff.state}` : "",
  }
}
