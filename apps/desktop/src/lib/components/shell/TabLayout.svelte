<script lang="ts">
  import type { Snippet } from "svelte"

  type Props = { aside?: Snippet; children: Snippet }

  let { aside, children }: Props = $props()
</script>

<div class="flex h-full gap-2 p-2 pt-0">
  {#if aside}
    {@render aside()}
  {/if}

  {@render children()}
</div>
