import { fail, redirect } from "@sveltejs/kit"

import { auth } from "$lib/server/auth"
import type { DesktopHandoff } from "$lib/types"

import type { Actions, PageServerLoad } from "./$types"

const HEX = new Set("0123456789abcdef")

function loopback(
  port: FormDataEntryValue | null,
  state: FormDataEntryValue | null,
): DesktopHandoff | null {
  const number = Number(port)

  if (!Number.isInteger(number) || number < 1024 || number > 65535) {
    return null
  }

  if (
    typeof state !== "string" ||
    state.length !== 32 ||
    ![...state].every(c => HEX.has(c))
  ) {
    return null
  }

  return { port: number, state }
}

function sameSite(path: string | null, origin: string) {
  if (!path?.startsWith("/")) {
    return null
  }

  const target = URL.parse(path, origin)

  return target?.origin === origin ? target.pathname + target.search : null
}

export const load: PageServerLoad = async ({ request, url, setHeaders }) => {
  setHeaders({ "x-frame-options": "DENY" })

  const session = await auth.api.getSession({ headers: request.headers })
  const next = sameSite(url.searchParams.get("next"), url.origin)

  if (session && next) {
    redirect(303, next)
  }

  return {
    account: session
      ? { name: session.user.name, email: session.user.email }
      : null,
    handoff: loopback(
      url.searchParams.get("port"),
      url.searchParams.get("state"),
    ),
    post: url.searchParams.get("form") === "1",
    next,
  }
}

export const actions: Actions = {
  connect: async ({ request }) => {
    const session = await auth.api.getSession({ headers: request.headers })

    if (!session) {
      return fail(401, { message: "Sign in first." })
    }

    const form = await request.formData()
    const handoff = loopback(form.get("port"), form.get("state"))

    if (!handoff) {
      return fail(400, { message: "This desktop link is not valid." })
    }

    const context = await auth.$context
    const desktop = await context.internalAdapter.createSession(
      session.user.id,
      false,
      { userAgent: "GPQL desktop" },
    )

    return {
      desktop: {
        ...handoff,
        token: desktop.token,
        post: form.get("form") === "1",
      },
    }
  },
}
