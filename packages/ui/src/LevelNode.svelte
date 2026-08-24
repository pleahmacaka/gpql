<script lang="ts">
  import type { NodeProps } from "@xyflow/svelte"

  let { data }: NodeProps = $props()
</script>

<p class="w-72 text-xs text-base-content/45">{data.label}</p>
