const DELAY = 300
const GAP = 6
const EDGE = 8

let box: HTMLDivElement | null = null

function ensure() {
  if (!box) {
    box = document.createElement("div")
    box.className = "tip"
    document.body.appendChild(box)
  }

  return box
}

export function tooltip(node: HTMLElement, text: string | null | undefined) {
  let current = text
  let timer: ReturnType<typeof setTimeout> | undefined

  function place() {
    if (!current) {
      return
    }

    const tip = ensure()

    tip.textContent = current
    tip.style.visibility = "hidden"
    tip.style.display = "block"

    const rect = node.getBoundingClientRect()
    const { offsetWidth: width, offsetHeight: height } = tip
    const room = window.innerHeight - rect.bottom - GAP - EDGE

    const x = Math.min(
      Math.max(rect.left + rect.width / 2 - width / 2, EDGE),
      window.innerWidth - width - EDGE,
    )
    const y = height > room ? rect.top - height - GAP : rect.bottom + GAP

    tip.style.left = `${x}px`
    tip.style.top = `${y}px`
    tip.style.visibility = "visible"
  }

  function hide() {
    clearTimeout(timer)

    if (box) {
      box.style.display = "none"
    }
  }

  function enter() {
    clearTimeout(timer)
    timer = setTimeout(place, DELAY)
  }

  const events = ["mouseenter", "focusin"] as const

  for (const name of events) {
    node.addEventListener(name, enter)
  }

  for (const name of ["mouseleave", "focusout", "pointerdown"] as const) {
    node.addEventListener(name, hide)
  }

  return {
    update(next: string | null | undefined) {
      current = next
    },
    destroy() {
      hide()

      for (const name of events) {
        node.removeEventListener(name, enter)
      }

      for (const name of ["mouseleave", "focusout", "pointerdown"] as const) {
        node.removeEventListener(name, hide)
      }
    },
  }
}
