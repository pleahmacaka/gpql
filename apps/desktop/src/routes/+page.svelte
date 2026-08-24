<script lang="ts">
  import DataTab from "$lib/components/data/DataTab.svelte"
  import QueryTab from "$lib/components/query/QueryTab.svelte"
  import SchemaTab from "$lib/components/schema/SchemaTab.svelte"
  import ConnectPanel from "$lib/components/session/ConnectPanel.svelte"
  import SessionMenu from "$lib/components/session/SessionMenu.svelte"
  import SettingsDialog from "$lib/components/settings/SettingsDialog.svelte"
  import FirstRun from "$lib/components/shell/FirstRun.svelte"
  import QuickActions from "$lib/components/shell/QuickActions.svelte"
  import TitleBar from "$lib/components/shell/TitleBar.svelte"
  import "@gpql/ui"
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
    root.style.setProperty(
      "--texture-amount",
      String(workspace.acrylic ? workspace.texture / 100 : 0),
    )
  })
</script>

<svelte:window
  onkeydown={event => {
    if (event.key === "Escape") {
      menuOpen = false
      settingsOpen = false
      paletteOpen = false
    }

    if (event.key === "k" && event.ctrlKey) {
      event.preventDefault()
      paletteOpen = !paletteOpen
    }
  }}
/>

<div class="relative flex h-full flex-col">
  <TitleBar
    {menuOpen}
    onToggleMenu={() => (menuOpen = !menuOpen)}
    onOpenSettings={() => (settingsOpen = true)}
  />

  <main class="min-h-0 flex-1">
    {#if !workspace.settled}
      <FirstRun ondone={() => workspace.settle()} />
    {:else if !workspace.session}
      <div class="h-full overflow-y-auto">
        <div class="mx-auto w-96 py-10">
          <ConnectPanel />
        </div>
      </div>
    {:else if workspace.tab === "data"}
      <DataTab />
    {:else if workspace.tab === "query"}
      <QueryTab />
    {:else}
      <SchemaTab />
    {/if}
  </main>

  {#if menuOpen}
    <SessionMenu onclose={() => (menuOpen = false)} />
  {/if}

  {#if settingsOpen}
    <SettingsDialog onclose={() => (settingsOpen = false)} />
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
