<script lang="ts">
  import { onMount } from "svelte"

  import { ConfirmDialog, EmptyState, Lazy, Panel } from "@gpql/ui"

  import ChatSurface from "$lib/components/agent/ChatSurface.svelte"
  import DataTab from "$lib/components/data/DataTab.svelte"
  import WritePreview from "$lib/components/data/WritePreview.svelte"
  import { newDiagram, openDiagram } from "$lib/components/erd/files"
  import ConnectPanel from "$lib/components/session/ConnectPanel.svelte"
  import {
    launcher,
    openDatabase,
  } from "$lib/components/session/launcher.svelte"
  import SessionMenu from "$lib/components/session/SessionMenu.svelte"
  import FirstRun from "$lib/components/shell/FirstRun.svelte"
  import QuickActions from "$lib/components/shell/QuickActions.svelte"
  import TitleBar from "$lib/components/shell/TitleBar.svelte"
  import Toasts from "$lib/components/shell/Toasts.svelte"
  import * as m from "$lib/paraglide/messages"
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

  let broken = $state("")

  onMount(() => {
    workspace.boot().catch(failure => (broken = String(failure)))
  })

  $effect(() => {
    const root = document.documentElement

    root.dataset.theme = workspace.theme
    root.dataset.acrylic = workspace.acrylic ? "on" : "off"
    root.dataset.motion = workspace.motion ? "full" : "calm"
  })

  let grain = $derived((workspace.texture / 100) * 0.2)

  let live = $derived(
    !!workspace.session && !workspace.connecting && !workspace.erd,
  )

  let launching = $derived(
    !workspace.session || workspace.connecting || workspace.adding,
  )

  let welcoming = $derived(
    launching && !workspace.erd && workspace.settled && !broken,
  )

  type Shortcut = {
    code: string
    ctrl?: boolean
    shift?: boolean
    bare?: boolean
    quiet?: boolean
    when?: () => boolean
    run: () => void
  }

  const SHORTCUTS: Shortcut[] = [
    {
      code: "KeyF",
      ctrl: true,
      when: () => live,
      run: () => (workspace.finding = true),
    },
    {
      code: "KeyJ",
      ctrl: true,
      when: () => workspace.agentReady,
      run: () => workspace.chat.show("panel"),
    },
    {
      code: "Space",
      ctrl: true,
      when: () => workspace.agentReady,
      run: () => workspace.chat.show("orb"),
    },
    { code: "Comma", ctrl: true, run: () => (settingsOpen = true) },
    { code: "KeyN", ctrl: true, run: () => launcher.open("new") },
    ...[
      { code: "KeyL", run: () => launcher.open("quick") },
      { code: "KeyH", run: () => launcher.open("recent") },
      { code: "KeyD", shift: true, run: newDiagram },
      { code: "KeyO", run: openDiagram },
      { code: "KeyO", shift: true, run: openDatabase },
    ].map(shortcut => ({
      ...shortcut,
      ctrl: true,
      quiet: true,
      when: () => welcoming,
    })),
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
    workspace.ddl = null

    if (workspace.chat.dock === "orb") {
      workspace.chat.dock = "off"
    }
  }

  // only draggable="true" elements drag; images, links and text stay in place
  function dragStart(event: DragEvent) {
    const from = event.target instanceof Element ? event.target : null

    if (!from?.closest("[draggable='true']")) {
      event.preventDefault()
    }
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "Escape" && !event.isComposing) {
      escape()

      return
    }

    const target = event.target
    const typing =
      event.defaultPrevented ||
      (target instanceof HTMLElement &&
        (["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) ||
          target.isContentEditable))

    for (const shortcut of SHORTCUTS) {
      if (event.code !== shortcut.code) {
        continue
      }

      if (shortcut.ctrl && !event.ctrlKey) {
        continue
      }

      if (!!shortcut.shift !== event.shiftKey && shortcut.ctrl) {
        continue
      }

      if (shortcut.bare && (event.ctrlKey || event.altKey || event.metaKey)) {
        continue
      }

      if (
        typing &&
        (shortcut.bare || shortcut.quiet || shortcut.code === "Space")
      ) {
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
  ondragstart={dragStart}
  onkeydown={keys}
/>

<div
  aria-hidden="true"
  class="pointer-events-none fixed inset-x-0 top-10 bottom-0 -z-10"
>
  <div class="grid-field absolute inset-0"></div>
  <div class="vignette absolute inset-0"></div>
  <div class="grain absolute inset-0" style:opacity={grain}></div>
</div>

{#key workspace.locale}
  <div class="relative flex h-full flex-col">
    <TitleBar
      {menuOpen}
      onToggleMenu={() => (menuOpen = !menuOpen)}
      onOpenSettings={() => (settingsOpen = true)}
      onOpenPalette={() => (paletteOpen = true)}
    />

    <div class="flex min-h-0 flex-1">
      <main class="min-h-0 min-w-0 flex-1">
        {#if broken}
          <div class="grid h-full place-items-center p-6">
            <Panel class="w-lg max-w-full">
              <EmptyState
                art="link"
                title={m.boot_failed()}
                hint={broken}
                class="select-text"
              />
            </Panel>
          </div>
        {:else if !workspace.settled}
          <FirstRun ondone={() => workspace.settle()} />
        {:else if workspace.erd}
          <Lazy
            load={() => import("$lib/components/erd/ErdEditor.svelte")}
            props={{ doc: workspace.erd }}
          />
        {:else if launching}
          <ConnectPanel />
        {:else}
          <div class={["h-full", workspace.tab !== "data" && "hidden"]}>
            <DataTab />
          </div>

          {#if visited.has("query")}
            <div class={["h-full", workspace.tab !== "query" && "hidden"]}>
              <Lazy
                load={() => import("$lib/components/query/QueryTab.svelte")}
              />
            </div>
          {/if}

          {#if visited.has("schema")}
            <div class={["h-full", workspace.tab !== "schema" && "hidden"]}>
              <Lazy
                load={() => import("$lib/components/schema/SchemaTab.svelte")}
              />
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

    {#if workspace.closing?.kind === "erd"}
      <ConfirmDialog
        title={m.erd_unsaved_title({ name: workspace.closing.label })}
        body={m.erd_unsaved_body()}
        confirm={m.erd_save()}
        alternate={m.erd_unsaved_confirm()}
        cancel={m.cancel()}
        icon="lucide:file-warning"
        tone="warning"
        onconfirm={() => workspace.saveAndClose()}
        onalternate={() => workspace.confirmClose()}
        oncancel={() => workspace.cancelClose()}
      />
    {:else if workspace.closing}
      <ConfirmDialog
        title={m.close_tx_title({ name: workspace.closing.label })}
        body={m.close_tx_body()}
        confirm={m.close_tx_confirm()}
        cancel={m.cancel()}
        icon="lucide:git-commit-horizontal"
        onconfirm={() => workspace.confirmClose()}
        oncancel={() => workspace.cancelClose()}
      />
    {/if}

    <Toasts />
  </div>
{/key}
