const FALLBACK = 16

// a virtualiser measures in device pixels, so sizes that belong to the spacing
// scale are declared in rem here and resolved once against the root font size
export function rem(multiple: number) {
  if (typeof document === "undefined") {
    return multiple * FALLBACK
  }

  const root = parseFloat(getComputedStyle(document.documentElement).fontSize)

  return multiple * (Number.isFinite(root) && root > 0 ? root : FALLBACK)
}
