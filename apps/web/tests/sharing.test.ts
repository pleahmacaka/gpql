import { expect, test } from "bun:test"

import { canSee } from "../src/lib/server/sharing"

const owned = { userId: "owner", open: 0 }

test("a private room opens for the person who published it", () => {
  expect(canSee(owned, "owner")).toBe("show")
})

test("a private room sends a signed out visitor to sign in", () => {
  expect(canSee(owned, null)).toBe("sign-in")
})

test("a private room stays hidden from another signed in account", () => {
  expect(canSee(owned, "someone-else")).toBe("hide")
})

test("an open room needs no account at all", () => {
  expect(canSee({ userId: "owner", open: 1 }, null)).toBe("show")
  expect(canSee({ userId: "owner", open: 1 }, "someone-else")).toBe("show")
})
