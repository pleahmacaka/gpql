export type MenuItem = {
  label: string
  icon?: string
  danger?: boolean
  run: () => void
}

class Menu {
  at = $state<{ x: number; y: number; items: MenuItem[] } | null>(null)

  show(event: MouseEvent, items: MenuItem[]) {
    event.preventDefault()
    event.stopPropagation()

    this.at = { x: event.clientX, y: event.clientY, items }
  }

  close = () => {
    this.at = null
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
