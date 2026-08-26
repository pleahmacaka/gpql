import { createAuthClient } from "better-auth/svelte"

export const authClient = createAuthClient()

export const signIn = (provider: "github") =>
  authClient.signIn.social({ provider, callbackURL: "/account" })

export const signOut = () => authClient.signOut()
