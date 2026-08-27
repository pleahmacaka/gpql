export function drag(
  event: PointerEvent,
  move: (event: PointerEvent) => void,
  done?: () => void,
) {
  const node = event.currentTarget as HTMLElement

  event.preventDefault()
  node.setPointerCapture(event.pointerId)

  const stop = () => {
    if (node.hasPointerCapture(event.pointerId)) {
      node.releasePointerCapture(event.pointerId)
    }

    node.removeEventListener("pointermove", move)
    node.removeEventListener("pointerup", stop)
    node.removeEventListener("pointercancel", stop)
    done?.()
  }

  node.addEventListener("pointermove", move)
  node.addEventListener("pointerup", stop)
  node.addEventListener("pointercancel", stop)
}
