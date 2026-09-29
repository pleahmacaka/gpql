<script lang="ts">
  import WindowChrome from "@gpql/ui/session/WindowChrome.svelte"
  import type { Snippet } from "svelte"

  import { passWheel } from "./wheel"

  type Props = {
    chip: string
    chipIcon: string
    tone: "idle" | "live"
    tab: string | null
    onchip?: () => void
    ontab?: (tab: string) => void
    children: Snippet
  }

  let { chip, chipIcon, tone, tab, onchip, ontab, children }: Props = $props()
</script>

<div
  class={[
    "hud flex h-full flex-col bg-base-100/70 p-2 hairline",
    "hover:hud-lit data-engaged:hud-lit",
  ]}
  {@attach passWheel}
>
  <div class="flex min-h-0 flex-1 flex-col overflow-x-auto bg-base-200">
    <div class="flex min-h-0 min-w-160 flex-1 flex-col lg:min-w-0">
      <WindowChrome {chip} {chipIcon} {tone} {tab} {onchip} {ontab} />

      <div class="grid min-h-0 flex-1 grid-cols-1 grid-rows-1">
        {@render children()}
      </div>
    </div>
  </div>
</div>
