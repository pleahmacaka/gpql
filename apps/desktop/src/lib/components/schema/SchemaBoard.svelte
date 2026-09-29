<script lang="ts">
  import { board, distinct, SchemaBoard } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  import { boardWords } from "./words"

  type Props = { keyboard: boolean }

  let { keyboard }: Props = $props()

  $effect(() => {
    board.selected = workspace.browse.table
  })

  $effect(() => {
    const label = workspace.session?.label

    if (!label) {
      return
    }

    let current = true

    workspace.loadLayout(label).then(saved => {
      if (current) {
        board.spots = saved.spots
        board.groups = distinct(saved.groups)
      }
    })

    return () => {
      current = false
    }
  })

  $effect(() => {
    board.onopen = table => {
      workspace.tab = "data"
      workspace.select(table)
    }
    board.ondefine = table => workspace.showDdl(table)

    return () => {
      board.onopen = null
      board.ondefine = null
    }
  })

  async function think(signal: AbortSignal) {
    const provider = workspace.model

    if (!workspace.ai || !provider) {
      throw new Error(m.ai_off_hint())
    }

    const { suggestGroups } = await import("$lib/ai/grouping")

    return await suggestGroups(workspace.schema, provider, signal)
  }
</script>

<SchemaBoard
  tables={workspace.schema}
  dark={workspace.dark}
  labels={boardWords()}
  {keyboard}
  minimap={workspace.minimap}
  onselect={table => workspace.select(table)}
  onsuggest={workspace.aiGroups ? think : undefined}
  onlayout={layout => workspace.saveLayout(layout)}
/>
