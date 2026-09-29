<script lang="ts">
  import type { Snippet } from "svelte"

  import Marker from "./Marker.svelte"

  type Props = {
    label?: string
    note?: string
    class?: string
    actions?: Snippet
    children: Snippet
  }

  let { label, note, class: extra = "", actions, children }: Props = $props()
</script>

<section class={["flex flex-col gap-2", extra]}>
  {#if label || actions}
    <div class="flex items-center gap-2">
      {#if label}
        <Marker as="h3" {label} class="flex-1" />
      {/if}

      {@render actions?.()}
    </div>
  {/if}

  {#if note}
    <p class="text-xs text-pretty text-base-content/70">{note}</p>
  {/if}

  <div class="flex flex-col divide-y divide-base-content/10 hairline">
    {@render children()}
  </div>
</section>
