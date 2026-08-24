<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { OptionRow } from "@gpql/ui"

  import { workspace } from "$lib/session/workspace.svelte"
</script>

<div class="space-y-2">
  <OptionRow
    icon="lucide:radar"
    title={m.option_scan()}
    detail={m.option_scan_hint()}
    on={workspace.autoscan}
    onclick={() => workspace.toggle("autoscan")}
  />
</div>
