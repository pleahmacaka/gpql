const ENDS = ["/", "\\", "?", "#"]

export function withoutLogin(address: string) {
  let shown = address
  let from = 0

  for (;;) {
    const scheme = shown.indexOf("://", from)

    if (scheme === -1) {
      return shown
    }

    const head = scheme + 3
    const stops = ENDS.map(mark => shown.indexOf(mark, head)).filter(
      at => at !== -1,
    )
    const end = stops.length > 0 ? Math.min(...stops) : shown.length
    const at = shown.lastIndexOf("@", end - 1)

    if (at >= head) {
      shown = shown.slice(0, head) + shown.slice(at + 1)
    }

    from = head
  }
}
