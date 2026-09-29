import { rem } from "@gpql/ui/controls/rem.ts"
import { calm } from "@gpql/ui/motion/index.ts"
import type { Attachment } from "svelte/attachments"

function unit(event: WheelEvent) {
  if (event.deltaMode === WheelEvent.DOM_DELTA_LINE) {
    return rem(2.5)
  }

  if (event.deltaMode === WheelEvent.DOM_DELTA_PAGE) {
    return innerHeight
  }

  return 1
}

export const passWheel: Attachment<HTMLElement> = node => {
  let engaged = false
  let goal = 0
  let frame = 0

  const glide = () => {
    const before = scrollY
    const gap = goal - before

    if (Math.abs(gap) < 1) {
      frame = 0

      return
    }

    const next = Math.abs(gap) < 4 ? goal : before + gap * 0.3

    scrollTo({ top: next, behavior: "instant" })

    if (scrollY === before) {
      frame = 0

      return
    }

    frame = requestAnimationFrame(glide)
  }

  const wheel = (event: WheelEvent) => {
    const sideways = Math.abs(event.deltaX) > Math.abs(event.deltaY)
    const modal = node.querySelector("[aria-modal='true']") !== null

    if (engaged || modal || event.ctrlKey || event.shiftKey || sideways) {
      return
    }

    event.preventDefault()
    event.stopPropagation()

    const limit = document.documentElement.scrollHeight - innerHeight
    const from = frame ? goal : scrollY

    goal = Math.max(0, Math.min(limit, from + event.deltaY * unit(event)))

    if (calm()) {
      scrollTo({ top: goal, behavior: "instant" })

      return
    }

    frame ||= requestAnimationFrame(glide)
  }

  const engage = () => {
    engaged = true
    node.toggleAttribute("data-engaged", true)
  }

  const release = () => {
    engaged = false
    node.toggleAttribute("data-engaged", false)
  }

  node.addEventListener("pointerdown", engage)
  node.addEventListener("pointerleave", release)
  node.addEventListener("wheel", wheel, { capture: true, passive: false })

  return () => {
    cancelAnimationFrame(frame)
    node.removeEventListener("pointerdown", engage)
    node.removeEventListener("pointerleave", release)
    node.removeEventListener("wheel", wheel, { capture: true })
  }
}
