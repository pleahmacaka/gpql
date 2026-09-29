<script lang="ts">
  import type { Snippet } from "svelte"

  import { Icon } from "../icons"

  type Props = {
    icon: string
    title: string
    detail: string
    children: Snippet
  }

  let { icon, title, detail, children }: Props = $props()
</script>

<div class="flex items-center gap-4 px-4 py-3">
  <Icon {icon} class="size-4 shrink-0 text-base-content/70" />

  <div class="min-w-0 flex-1">
    <p class="text-sm">{title}</p>
    <p class="text-xs text-base-content/70">{detail}</p>
  </div>

  <div class="flex shrink-0 items-center gap-2">
    {@render children()}
  </div>
</div>
