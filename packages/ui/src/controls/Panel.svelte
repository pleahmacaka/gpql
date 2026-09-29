<script lang="ts">
  import type { Snippet } from "svelte"

  type Props = {
    glass?: boolean
    lit?: boolean
    wide?: boolean
    label?: string
    class?: string
    inner?: string
    children: Snippet
  }

  let {
    glass = true,
    lit = false,
    wide = false,
    label,
    class: outer = "",
    inner = "",
    children,
  }: Props = $props()
</script>

<section
  aria-label={label}
  class={["group/panel relative flex min-h-0 flex-col", outer]}
>
  <div
    aria-hidden="true"
    class={[
      "pointer-events-none absolute inset-0",
      glass ? "pane" : "bg-base-100 hairline",
    ]}
  ></div>

  <div class={["relative flex min-h-0 flex-1 flex-col", inner]}>
    {@render children()}
  </div>

  <div
    aria-hidden="true"
    class={[
      "hud pointer-events-none absolute inset-0",
      "group-focus-within/panel:hud-lit",
      wide && "hud-wide",
      lit && "hud-lit",
    ]}
  ></div>
</section>
