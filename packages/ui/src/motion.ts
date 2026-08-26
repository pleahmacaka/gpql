export function calm() {
  if (typeof document === "undefined") {
    return true
  }

  return document.documentElement.dataset.motion === "calm"
}

export function veil() {
  return { duration: calm() ? 0 : 120 }
}

export function pop() {
  return { duration: calm() ? 0 : 140, start: 0.97 }
}
