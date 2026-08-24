import type { Handle } from "@sveltejs/kit"
import { svelteKitHandler } from "better-auth/svelte-kit"
import { building } from "$app/environment"

import { auth } from "$lib/server/auth"

export const handle: Handle = async ({ event, resolve }) =>
  svelteKitHandler({ event, resolve, auth, building })
