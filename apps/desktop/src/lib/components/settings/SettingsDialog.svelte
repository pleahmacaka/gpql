<script lang="ts">
  import { Dialog } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"

  import SettingsView from "./SettingsView.svelte"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()
</script>

<Dialog
  label={m.settings()}
  {onclose}
  dismiss={m.close()}
  size="xl"
  class="h-160"
>
  <SettingsView {onclose} class="min-h-0 flex-1" />
</Dialog>
