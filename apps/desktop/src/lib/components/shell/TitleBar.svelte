<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { WindowChrome } from "@gpql/ui"

  import { workspace } from "$lib/session/workspace.svelte"
  import type { Tab } from "$lib/types"

  import WindowControls from "./WindowControls.svelte"

  type Props = {
    menuOpen: boolean
    onToggleMenu: () => void
    onOpenSettings: () => void
  }

  let { onToggleMenu, onOpenSettings }: Props = $props()

  let label = $derived(
    workspace.tab.charAt(0).toUpperCase() + workspace.tab.slice(1),
  )

  async function pick(label: string) {
    const tab = label.toLowerCase() as Tab

    workspace.tab = tab

    if (tab === "schema") {
      await workspace.loadSchema()
    }
  }
</script>

<WindowChrome
  chip={workspace.erd?.name ?? workspace.session?.label ?? m.no_session()}
  chipIcon={workspace.erd
    ? "lucide:git-fork"
    : workspace.session
      ? workspace.iconFor(workspace.session.kind)
      : "lucide:plus"}
  tab={workspace.session && !workspace.erd ? label : null}
  live
  onchip={onToggleMenu}
  ontab={pick}
  onsettings={onOpenSettings}
  agentOn={workspace.chat.dock === "panel"}
  onagent={workspace.agentReady
    ? () => workspace.chat.show("panel")
    : undefined}
>
  {#snippet controls()}
    <WindowControls />
  {/snippet}
</WindowChrome>
