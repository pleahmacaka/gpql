import { cubicIn, cubicOut } from "svelte/easing"
import type { TransitionConfig } from "svelte/transition"

export type Direction = "left" | "right" | "up" | "down" | "center"

export type Ending = "confirm" | "cancel"

export const TIMING = { quick: 140, base: 200, slow: 260 }

const SHIFT: Record<Direction, [number, number]> = {
  left: [-1, 0],
  right: [1, 0],
  up: [0, -1],
  down: [0, 1],
  center: [0, 0],
}

export function calm() {
  if (typeof document === "undefined") {
    return true
  }

  return (
    document.documentElement.dataset.motion === "calm" ||
    matchMedia("(prefers-reduced-motion: reduce)").matches
  )
}

const faint = (): TransitionConfig => ({
  duration: 80,
  css: t => `opacity: ${t}`,
})

function travel(side: Direction, distance: number, t: number, u: number) {
  const [x, y] = SHIFT[side]
  const zoom = side === "center" ? 0.98 + 0.02 * t : 1

  return (
    `opacity: ${t}; ` +
    `transform: translate(${x * u * distance}rem, ${y * u * distance}rem) ` +
    `scale(${zoom})`
  )
}

export function veil() {
  return { duration: calm() ? 0 : TIMING.quick }
}

export function pop() {
  return { duration: calm() ? 0 : TIMING.quick, start: 0.97 }
}

export function rise(_node: Element): TransitionConfig {
  if (calm()) {
    return faint()
  }

  return {
    duration: 180,
    easing: cubicOut,
    css: (t, u) =>
      `opacity: ${t}; transform: translateY(${-0.25 * u}rem) ` +
      `scale(${0.97 + 0.03 * t})`,
  }
}

export function leave(
  _node: Element,
  { as = "cancel" }: { as?: Ending } = {},
): TransitionConfig {
  if (calm()) {
    return faint()
  }

  if (as === "confirm") {
    return {
      duration: 180,
      easing: cubicIn,
      css: (t, u) => `opacity: ${t}; transform: scale(${1 + 0.015 * u})`,
    }
  }

  return {
    duration: TIMING.quick,
    easing: cubicIn,
    css: (t, u) =>
      `opacity: ${t}; transform: translateY(${0.25 * u}rem) ` +
      `scale(${1 - 0.03 * u})`,
  }
}

export function arrive(
  _node: Element,
  {
    from = "center",
    distance = 2,
    delay = 0,
  }: { from?: Direction; distance?: number; delay?: number } = {},
): TransitionConfig {
  if (calm()) {
    return faint()
  }

  return {
    delay,
    duration: TIMING.slow,
    easing: cubicOut,
    css: (t, u) => travel(from, distance, t, u),
  }
}

export function depart(
  _node: Element,
  {
    to = "center",
    distance = 2,
    delay = 0,
  }: { to?: Direction; distance?: number; delay?: number } = {},
): TransitionConfig {
  if (calm()) {
    return faint()
  }

  return {
    delay,
    duration: TIMING.base,
    easing: cubicIn,
    css: (t, u) => travel(to, distance, t, u),
  }
}
