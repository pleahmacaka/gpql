<script lang="ts">
  import DataTab from "$lib/components/data/DataTab.svelte"
  import ConnectPanel from "$lib/components/session/ConnectPanel.svelte"
  import SessionMenu from "$lib/components/session/SessionMenu.svelte"
  import FirstRun from "$lib/components/shell/FirstRun.svelte"
  import QuickActions from "$lib/components/shell/QuickActions.svelte"
  import TitleBar from "$lib/components/shell/TitleBar.svelte"
  import { Lazy } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"
  import { onMount } from "svelte"

  let menuOpen = $state(false)
  let settingsOpen = $state(false)
  let paletteOpen = $state(false)

  onMount(() => {
    workspace.boot()
  })

  $effect(() => {
    const root = document.documentElement

    root.dataset.theme = workspace.theme
    root.dataset.acrylic = workspace.acrylic ? "on" : "off"
    root.dataset.motion = workspace.motion ? "full" : "calm"
    root.style.setProperty(
      "--texture-amount",
      String(workspace.acrylic ? workspace.texture / 100 : 0),
    )
  })
</script>

<svelte:window
  oncontextmenu={event => event.preventDefault()}
  ondragstart={event => event.preventDefault()}
  onkeydown={event => {
    if (event.key === "Escape") {
      menuOpen = false
      settingsOpen = false
      paletteOpen = false
    }

    if (event.key === "j" && event.ctrlKey) {
      event.preventDefault()
      workspace.chatOpen = !workspace.chatOpen
    }

    if (event.key === "," && event.ctrlKey) {
      event.preventDefault()
      settingsOpen = true
    }

    if (event.key === "k" && event.ctrlKey) {
      event.preventDefault()
      paletteOpen = !paletteOpen
    }
  }}
/>

{#key workspace.locale}
<div class="relative flex h-full flex-col">
  <TitleBar
    {menuOpen}
    onToggleMenu={() => (menuOpen = !menuOpen)}
    onOpenSettings={() => (settingsOpen = true)}
  />

  <div class="flex min-h-0 flex-1">
    <main class="min-h-0 min-w-0 flex-1 gridfield">
    {#if !workspace.settled}
      <FirstRun ondone={() => workspace.settle()} />
    {:else if workspace.erd}
      <Lazy
        load={() => import("$lib/components/erd/ErdEditor.svelte")}
        props={{ doc: workspace.erd }}
      />
    {:else if !workspace.session || workspace.connecting}
      <div class="h-full overflow-y-auto">
        <div class="mx-auto w-96 py-10">
          <ConnectPanel />
        </div>
      </div>
    {:else if workspace.tab === "data"}
      <DataTab />
    {:else if workspace.tab === "query"}
      <Lazy load={() => import("$lib/components/query/QueryTab.svelte")} />
    {:else}
      <Lazy load={() => import("$lib/components/schema/SchemaTab.svelte")} />
    {/if}
    </main>

    {#if workspace.ai && workspace.chatOpen}
      <div class="min-h-0 py-2 pr-2">
        <Lazy load={() => import("$lib/components/agent/AgentChat.svelte")} />
      </div>
    {/if}
  </div>

  {#if menuOpen}
    <SessionMenu onclose={() => (menuOpen = false)} />
  {/if}

  {#if settingsOpen}
    <Lazy
      load={() => import("$lib/components/settings/SettingsDialog.svelte")}
      props={{ onclose: () => (settingsOpen = false) }}
    />
  {/if}

  {#if paletteOpen}
    <QuickActions
      onclose={() => (paletteOpen = false)}
      onsettings={() => (settingsOpen = true)}
    />
  {/if}

  {#if workspace.acrylic && workspace.texture > 0}
    <div class="texture" aria-hidden="true"></div>
  {/if}
</div>
{/key}
