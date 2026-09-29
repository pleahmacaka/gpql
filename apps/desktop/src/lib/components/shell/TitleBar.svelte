<script lang="ts">
  import { Icon, tooltip, WindowChrome } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Tab } from "$lib/types"

  import WindowControls from "./WindowControls.svelte"

  type Props = {
    menuOpen: boolean
    onToggleMenu: () => void
    onOpenSettings: () => void
    onOpenPalette: () => void
  }

  let { menuOpen, onToggleMenu, onOpenSettings, onOpenPalette }: Props =
    $props()

  const TABS: Tab[] = ["data", "query", "schema"]

  let writing = $derived(!!workspace.session && !workspace.readOnly)
  let pending = $derived(!!workspace.active?.openTransaction)

  let browsing = $derived(
    !!workspace.session &&
      !workspace.erd &&
      !workspace.connecting &&
      !workspace.adding,
  )

  async function pick(id: string) {
    const tab = TABS.find(entry => entry === id)

    if (!tab) {
      return
    }

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
  chipOpen={menuOpen}
  tone={!workspace.session ? "idle" : writing ? "warning" : "live"}
  tab={browsing ? workspace.tab : null}
  live
  onchip={onToggleMenu}
  ontab={pick}
  onsettings={onOpenSettings}
  onpalette={onOpenPalette}
  agentOn={workspace.chat.dock === "panel"}
  onagent={workspace.agentReady
    ? () => workspace.chat.show("panel")
    : undefined}
  labels={{
    data: m.tab_data(),
    query: m.tab_query(),
    schema: m.tab_schema(),
    agent: m.agent(),
    settings: m.settings(),
    palette: m.action_palette(),
    tabs: m.tabs_label(),
  }}
>
  {#snippet status()}
    {#if pending}
      <span class="badge badge-sm badge-soft badge-warning gap-1">
        <Icon icon="lucide:git-commit-horizontal" class="size-3" />
        {m.tx_open()}
      </span>
    {/if}

    {#if writing}
      <button
        type="button"
        onclick={() => workspace.setReadOnly(true)}
        use:tooltip={m.writes_lock()}
        class={[
          "badge badge-sm badge-warning cursor-pointer gap-1 font-medium",
          "transition-opacity hover:opacity-80",
        ]}
      >
        <Icon icon="lucide:pencil" class="size-3" />
        {m.writes_on()}
      </button>
    {/if}
  {/snippet}

  {#snippet controls()}
    <WindowControls />
  {/snippet}
</WindowChrome>
