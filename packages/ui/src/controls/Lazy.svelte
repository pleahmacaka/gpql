<script lang="ts">
  import type { Component } from "svelte"

  type Props = {
    load: () => Promise<{ default: Component<never> }>
    props?: Record<string, unknown>
  }

  let { load, props = {} }: Props = $props()
</script>

{#await load() then module}
  {@const View = module.default as Component<Record<string, unknown>>}
  <View {...props} />
{/await}
