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

<div class="flex items-center gap-3 rounded-field bg-base-200 px-3 py-2.5">
  <Icon {icon} class="size-4 text-base-content/40" />

  <div class="flex-1">
    <p class="text-sm">{title}</p>
    <p class="text-xs text-base-content/45">{detail}</p>
  </div>

  {@render children()}
</div>
