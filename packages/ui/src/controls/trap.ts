const FOCUSABLE = [
  "button:not([disabled])",
  "a[href]",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[contenteditable]",
  '[tabindex]:not([tabindex="-1"])',
].join(", ")

export function trap(node: HTMLElement, onescape?: () => void) {
  const before =
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null
  let leave = onescape

  const stops = () =>
    [...node.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
      element => element.getClientRects().length > 0,
    )

  function keys(event: KeyboardEvent) {
    if (event.defaultPrevented) {
      return
    }

    if (event.key === "Escape" && leave && !event.isComposing) {
      event.preventDefault()
      event.stopPropagation()
      leave()

      return
    }

    if (event.key !== "Tab") {
      return
    }

    const items = stops()
    const first = items[0]
    const last = items[items.length - 1]
    const at = document.activeElement

    if (!first || !last) {
      event.preventDefault()
      node.focus()

      return
    }

    if (event.shiftKey && (at === first || !node.contains(at))) {
      event.preventDefault()
      last.focus()
    } else if (!event.shiftKey && (at === last || !node.contains(at))) {
      event.preventDefault()
      first.focus()
    }
  }

  node.addEventListener("keydown", keys)

  queueMicrotask(() => {
    if (!node.contains(document.activeElement)) {
      const start =
        node.querySelector<HTMLElement>("[autofocus], [data-autofocus]") ??
        stops()[0] ??
        node

      start.focus()
    }
  })

  return {
    update(next?: () => void) {
      leave = next
    },
    destroy() {
      node.removeEventListener("keydown", keys)

      const at = document.activeElement
      const unclaimed = at === null || at === document.body || node.contains(at)

      if (before?.isConnected && unclaimed) {
        before.focus()
      }
    },
  }
}
