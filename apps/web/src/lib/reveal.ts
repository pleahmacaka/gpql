export function reveal(node: HTMLElement) {
  if (typeof IntersectionObserver === "undefined") {
    node.dataset.shown = "yes"

    return
  }

  const watcher = new IntersectionObserver(
    entries => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          node.dataset.shown = "yes"
          watcher.unobserve(entry.target)
        }
      }
    },
    { rootMargin: "0px 0px -10% 0px", threshold: 0.05 },
  )

  watcher.observe(node)

  return {
    destroy() {
      watcher.disconnect()
    },
  }
}
