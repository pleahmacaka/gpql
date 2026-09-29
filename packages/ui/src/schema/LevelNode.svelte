<script lang="ts">
  import type { NodeProps } from "@xyflow/svelte"

  import Marker from "../controls/Marker.svelte"
  import { spoken } from "./board.svelte"

  let { data }: NodeProps = $props()

  const words = spoken()

  let label = $derived.by(() => {
    const said = words()
    const depth = data.depth as number
    const level = depth === 0 ? said.referenced : `${said.level} ${depth}`

    return data.grouped ? `${said.rest}, ${level}` : level
  })
</script>

<Marker {label} class="w-72" />
