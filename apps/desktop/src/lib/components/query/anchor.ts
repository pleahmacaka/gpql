import { type MenuItem, menu } from "@gpql/ui"

export function menuBelow(event: MouseEvent, items: MenuItem[]) {
  const target = event.currentTarget

  if (!(target instanceof HTMLElement)) {
    menu.show(event, items)

    return
  }

  const box = target.getBoundingClientRect()

  menu.show(
    new MouseEvent("click", { clientX: box.left, clientY: box.bottom }),
    items,
  )
  event.preventDefault()
  event.stopPropagation()
}
