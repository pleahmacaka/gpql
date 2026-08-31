export type Room = { userId: string; open: number }

export type Verdict = "show" | "sign-in" | "hide"

export function canSee(room: Room, viewerId: string | null): Verdict {
  if (room.open === 1) {
    return "show"
  }

  if (viewerId === null) {
    return "sign-in"
  }

  return viewerId === room.userId ? "show" : "hide"
}
