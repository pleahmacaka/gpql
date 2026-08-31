<script lang="ts">
  import { board, SchemaBoard } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  $effect(() => {
    board.selected = workspace.browse.table
  })

  $effect(() => {
    const label = workspace.session?.label

    if (!label) {
      return
    }

    workspace.loadLayout(label).then(saved => {
      board.spots = saved.spots
      board.groups = saved.groups
    })
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
  labels={{
    auto: m.arrange_auto(),
    picked: m.arrange_picked(),
    group: m.group_make(),
    ungroup: m.group_drop(),
    warn: m.arrange_warn(),
    groupName: m.group_name(),
    think: m.group_ai(),
    nothing: m.group_ai_none(),
    rest: m.group_rest(),
    define: m.menu_ddl(),
    cancel: m.cancel(),
  }}
  keyboard={workspace.tab === "schema"}
  minimap={workspace.minimap}
  onselect={table => workspace.select(table)}
  onsuggest={workspace.aiGroups ? think : undefined}
  onlayout={layout => workspace.saveLayout(layout)}
/>

