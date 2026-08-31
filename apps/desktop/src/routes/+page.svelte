<script lang="ts">
  import { onMount } from "svelte"

  import { fade } from "svelte/transition"

  import { Lazy, veil } from "@gpql/ui"

  import ChatSurface from "$lib/components/agent/ChatSurface.svelte"
  import DataTab from "$lib/components/data/DataTab.svelte"
  import DdlDialog from "$lib/components/data/DdlDialog.svelte"
  import WritePreview from "$lib/components/data/WritePreview.svelte"
  import ConnectPanel from "$lib/components/session/ConnectPanel.svelte"
  import SessionMenu from "$lib/components/session/SessionMenu.svelte"
  import FirstRun from "$lib/components/shell/FirstRun.svelte"
  import QuickActions from "$lib/components/shell/QuickActions.svelte"
  import TitleBar from "$lib/components/shell/TitleBar.svelte"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Tab } from "$lib/types"

  let menuOpen = $state(false)
  let settingsOpen = $state(false)
  let paletteOpen = $state(false)

  const TABS: Tab[] = ["data", "query", "schema"]

  let visited = $state(new Set<Tab>(["data"]))

  $effect(() => {
    if (!visited.has(workspace.tab)) {
      visited = new Set(visited).add(workspace.tab)
    }
  })

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

  let live = $derived(!!workspace.session && !workspace.connecting && !workspace.erd)

  type Shortcut = {
    code: string
    ctrl?: boolean
    bare?: boolean
    when?: () => boolean
    run: () => void
  }

  const SHORTCUTS: Shortcut[] = [
    { code: "KeyF", ctrl: true, when: () => live, run: () => (workspace.finding = true) },
    { code: "KeyJ", ctrl: true, when: () => workspace.agentReady, run: () => workspace.chat.show("panel") },
    { code: "Space", ctrl: true, when: () => workspace.agentReady, run: () => workspace.chat.show("orb") },
    { code: "Comma", ctrl: true, run: () => (settingsOpen = true) },
    { code: "KeyK", ctrl: true, run: () => (paletteOpen = !paletteOpen) },
    ...TABS.map((tab, index) => ({
      code: `Digit${index + 1}`,
      bare: true,
      when: () => live,
      run: () => (workspace.tab = tab),
    })),
  ]

  function escape() {
    menuOpen = false
    settingsOpen = false
    paletteOpen = false
    workspace.finding = false
    workspace.chat.dock = "off"
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "Escape") {
      escape()

      return
    }

    const target = event.target as HTMLElement
    const typing =
      event.defaultPrevented ||
      ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) ||
      target.isContentEditable

    for (const shortcut of SHORTCUTS) {
      if (event.code !== shortcut.code) {
        continue
      }

      if (shortcut.ctrl && !event.ctrlKey) {
        continue
      }

      if (shortcut.bare && (event.ctrlKey || event.altKey || event.metaKey)) {
        continue
      }

      if (typing && (shortcut.bare || shortcut.code === "Space")) {
        continue
      }

      if (shortcut.when && !shortcut.when()) {
        continue
      }

      event.preventDefault()
      shortcut.run()

      return
    }
  }
</script>

<svelte:window
  oncontextmenu={event => event.preventDefault()}
  ondragstart={event => event.preventDefault()}
  onkeydown={keys}
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
    {:else if !workspace.session || workspace.connecting || workspace.adding}
      <div class="h-full overflow-y-auto">
        <div class="mx-auto w-96 py-10">
          <ConnectPanel />
        </div>
      </div>
    {:else}
      <div class="h-full" class:hidden={workspace.tab !== "data"}>
        <DataTab />
      </div>

      {#if visited.has("query")}
        <div class="h-full" class:hidden={workspace.tab !== "query"}>
          <Lazy load={() => import("$lib/components/query/QueryTab.svelte")} />
        </div>
      {/if}

      {#if visited.has("schema")}
        <div class="h-full" class:hidden={workspace.tab !== "schema"}>
          <Lazy load={() => import("$lib/components/schema/SchemaTab.svelte")} />
        </div>
      {/if}
    {/if}
    </main>

    {#if workspace.agentReady && workspace.chat.dock === "panel"}
      <div class="flex min-h-0 py-2 pr-2">
        <ChatSurface />
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

  {#if workspace.agentReady && workspace.chat.dock === "orb"}
    <ChatSurface />
  {/if}

  {#if paletteOpen}
    <QuickActions
      onclose={() => (paletteOpen = false)}
      onsettings={() => (settingsOpen = true)}
    />
  {/if}

  <WritePreview />
  <DdlDialog />

  {#if workspace.notice}
    <button
      type="button"
      transition:fade={veil()}
      onclick={() => (workspace.notice = "")}
      class="fixed bottom-6 left-1/2 z-50 max-w-2xl -translate-x-1/2 rounded-box
        floating px-4 py-2 text-left text-xs text-base-content/80 lift"
    >
      {workspace.notice}
    </button>
  {/if}

  {#if workspace.acrylic && workspace.texture > 0}
    <div class="texture" aria-hidden="true"></div>
  {/if}
</div>
{/key}
