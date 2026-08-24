<script lang="ts">
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
  chip={workspace.session?.label ?? "no session"}
  chipIcon={workspace.session
    ? workspace.session.kind === "sqlite"
      ? "lucide:file"
      : "lucide:database"
    : "lucide:plus"}
  tab={workspace.session ? label : null}
  live
  onchip={onToggleMenu}
  ontab={pick}
  onsettings={onOpenSettings}
  agentOn={workspace.chatOpen}
  onagent={() => (workspace.chatOpen = !workspace.chatOpen)}
>
  {#snippet controls()}
    <WindowControls />
  {/snippet}
</WindowChrome>
