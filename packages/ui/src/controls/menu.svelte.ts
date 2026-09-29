export type MenuItem = {
  label: string
  icon?: string
  danger?: boolean
  run: () => void
}

class Menu {
  at = $state<{ x: number; y: number; items: MenuItem[] } | null>(null)

  private back: HTMLElement | null = null

  show(event: MouseEvent, items: MenuItem[]) {
    event.preventDefault()
    event.stopPropagation()

    if (!this.at) {
      const focused = document.activeElement

      this.back = focused instanceof HTMLElement ? focused : null
    }

    this.at = { x: event.clientX, y: event.clientY, items }
  }

  close = () => {
    const back = this.back

    this.at = null
    this.back = null
    back?.focus({ preventScroll: true })
  }
}

export const menu = new Menu()

export function contextmenu(node: HTMLElement, items: () => MenuItem[]) {
  let current = items

  const show = (event: MouseEvent) => menu.show(event, current())

  node.addEventListener("contextmenu", show)

  return {
    update(next: () => MenuItem[]) {
      current = next
    },
    destroy() {
      node.removeEventListener("contextmenu", show)
    },
  }
}
