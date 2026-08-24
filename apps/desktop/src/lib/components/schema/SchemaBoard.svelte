<script lang="ts">
  import { board, SchemaBoard } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  $effect(() => {
    board.selected = workspace.selected
  })
</script>

<SchemaBoard
  tables={workspace.schema}
  dark={workspace.dark}
  tidyLabel={m.tidy()}
  keyboard={workspace.tab === "schema"}
  onselect={table => workspace.select(table)}
/>
