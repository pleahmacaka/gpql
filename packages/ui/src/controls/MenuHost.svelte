<script lang="ts">
  import ContextMenu from "./ContextMenu.svelte"
  import { menu } from "./menu.svelte"
</script>

{#if menu.at}
  <ContextMenu
    x={menu.at.x}
    y={menu.at.y}
    items={menu.at.items}
    onclose={menu.close}
  />
{/if}
