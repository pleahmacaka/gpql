<script lang="ts">
  import { onDestroy, tick } from "svelte"

  import {
    arrive,
    ConfirmDialog,
    calm,
    depart,
    field,
    Icon,
    Keycap,
    leave,
    mark,
    rise,
    scramble,
    TIMING,
  } from "@gpql/ui"

  import { newDiagram, openDiagram } from "$lib/components/erd/files"
  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { SessionConfig } from "$lib/types"

  import ConnectionList from "./ConnectionList.svelte"
  import { launcher, openDatabase, type View } from "./launcher.svelte"
  import NewSession from "./NewSession.svelte"
  import QuickConnect from "./QuickConnect.svelte"

  type Action = {
    id: string
    label: string
    icon: string
    keys: string[]
    view?: View
    run?: () => Promise<void>
  }

  const LEFT = [1 / 12, 5 / 12, 9 / 12]
  const RIGHT = [3 / 12, 7 / 12, 11 / 12]
  const ROWS = {
    left: ["top-1/12", "top-5/12", "top-9/12"],
    right: ["top-3/12", "top-7/12", "top-11/12"],
  }
  const ORDER = ["new", "quick", "recent", "diagram", "open", "file"]
  const KEYS: Record<string, string> = { ctrl: "Control", shift: "Shift" }
  const STAGGER = 40

  const TARGETS = [
    "[data-card] [data-autofocus]",
    "[data-card] input",
    "[data-card-body] button:enabled",
  ].join(", ")

  const FRAME: Record<Exclude<View, "home">, string> = {
    new: "max-h-192 max-w-lg",
    quick: "max-h-160 max-w-md",
    recent: "max-h-160 max-w-lg",
  }

  let seed = $state<SessionConfig | null>(null)
  let selecting = $state<string | null>(null)
  let dropping = $state(false)
  let stage = $state<HTMLDivElement | null>(null)
  let timer: ReturnType<typeof setTimeout> | undefined

  let busy = $derived(workspace.busy || workspace.dialing !== null)
  let home = $derived(launcher.view === "home")

  const signature = mark({
    lanes: { left: LEFT, right: RIGHT },
    start: performance.now() / 1000,
    busy: () => busy,
    hot: () => (selecting ? ORDER.indexOf(selecting) : null),
  })

  let left = $derived<Action[]>([
    {
      id: "new",
      label: m.action_new(),
      icon: "lucide:plus",
      keys: ["ctrl", "n"],
      view: "new",
    },
    {
      id: "quick",
      label: m.action_quick(),
      icon: "lucide:radar",
      keys: ["ctrl", "l"],
      view: "quick",
    },
    {
      id: "recent",
      label: m.recent_connections(),
      icon: "lucide:history",
      keys: ["ctrl", "h"],
      view: "recent",
    },
  ])

  let right = $derived<Action[]>([
    {
      id: "diagram",
      label: m.erd_new(),
      icon: "lucide:file-plus-2",
      keys: ["ctrl", "shift", "d"],
      run: newDiagram,
    },
    {
      id: "open",
      label: m.erd_open(),
      icon: "lucide:folder-open",
      keys: ["ctrl", "o"],
      run: openDiagram,
    },
    {
      id: "file",
      label: m.action_open_file(),
      icon: "lucide:database",
      keys: ["ctrl", "shift", "o"],
      run: openDatabase,
    },
  ])

  let title = $derived(
    {
      home: "GPQL",
      new: workspace.editing ? m.menu_edit() : m.panel_new(),
      quick: m.panel_quick(),
      recent: m.recent_connections(),
    }[launcher.view],
  )

  const shortcut = (keys: string[]) =>
    keys.map(key => KEYS[key] ?? key.toUpperCase()).join("+")

  function select(entry: Action) {
    if (selecting || !home) {
      return
    }

    selecting = entry.id

    timer = setTimeout(
      async () => {
        if (entry.view) {
          selecting = null
          seed = null
          workspace.editing = null
          launcher.open(entry.view, entry.id)

          return
        }

        await entry.run?.()
        selecting = null
      },
      calm() ? 0 : TIMING.quick,
    )
  }

  function handoff(config: SessionConfig) {
    seed = config
    launcher.open("new", "new")
  }

  function fresh() {
    seed = null
    workspace.editing = null
    launcher.open("new", "new")
  }

  function back() {
    seed = null
    launcher.back()
  }

  function leaveLauncher() {
    workspace.connecting = false
    workspace.adding = false
  }

  function keys(event: KeyboardEvent) {
    if (
      event.key === "Escape" &&
      !event.isComposing &&
      !event.defaultPrevented &&
      !home
    ) {
      back()
    }
  }

  $effect(() => {
    const landing = home
    const origin = launcher.origin

    void tick().then(() => {
      const target = landing
        ? stage?.querySelector<HTMLElement>(`[data-action="${origin}"]`)
        : stage?.querySelector<HTMLElement>(TARGETS)

      target?.focus()
    })
  })

  onDestroy(() => {
    clearTimeout(timer)

    if (workspace.session && !workspace.connecting) {
      launcher.view = "home"
    }
  })
</script>

<svelte:window onkeydown={keys} />

{#snippet action(entry: Action, index: number, inward: boolean)}
  <div
    class={[
      "absolute -translate-y-1/2",
      inward ? "right-0" : "left-0",
      ROWS[inward ? "left" : "right"][index],
    ]}
  >
    <button
      type="button"
      data-action={entry.id}
      onclick={() => select(entry)}
      aria-keyshortcuts={shortcut(entry.keys)}
      in:arrive|global={{
        from: inward ? "right" : "left",
        distance: 1,
        delay: index * STAGGER,
      }}
      out:depart|global={{
        to: inward ? "right" : "left",
        distance: 1,
        delay: index * STAGGER,
      }}
      class={[
        "hud hud-small flex w-52 cursor-pointer items-center gap-3 px-4 py-3",
        "text-left transition-colors hover:hud-lit focus-visible:hud-lit",
        "lg:w-72",
        selecting === entry.id
          ? "hud-lit bg-primary text-primary-content"
          : entry.id === "new"
            ? "bg-primary/15 hover:bg-primary/25"
            : "bg-base-100 hairline hover:bg-base-300",
      ]}
    >
      <Icon
        icon={entry.icon}
        class={[
          "size-5 shrink-0",
          selecting === entry.id ? "text-primary-content" : "text-primary",
        ]}
      />

      <span class="min-w-0 flex-1 truncate text-sm font-medium">
        {entry.label}
      </span>

      <Keycap keys={entry.keys} class="hidden shrink-0 lg:flex" />
    </button>
  </div>
{/snippet}

<div bind:this={stage} class="relative h-full overflow-hidden bg-base-200/90">
  <div
    class={[
      "flex h-full flex-col items-center justify-center-safe gap-6",
      "overflow-y-auto px-8 py-6 tall:gap-8",
    ]}
  >
    <header
      inert={!home}
      class={[
        "flex flex-col items-center gap-1 text-center transition",
        "duration-260 ease-out",
        home ? "opacity-100" : "-translate-y-4 opacity-0",
      ]}
    >
      <h1
        use:scramble={{ duration: 700 }}
        class="text-3xl leading-tight font-bold tracking-tight"
      >
        GPQL
      </h1>

      <p class="text-sm leading-relaxed text-base-content/80">
        {m.launcher_tagline()}
      </p>

      <p
        class={[
          "flex items-center gap-4 text-xs text-base-content/70",
          "tabular-nums",
        ]}
      >
        <span>{m.about_version({ version: __GPQL_VERSION__ })}</span>
        <span>{m.backends_count({ count: workspace.catalog.length })}</span>
      </p>

      {#if workspace.session}
        <button
          type="button"
          onclick={leaveLauncher}
          class="btn btn-soft btn-sm mt-2 font-medium"
        >
          <Icon icon="lucide:arrow-left" class="size-4" />
          {m.launcher_back({ name: workspace.session.label })}
        </button>
      {/if}
    </header>

    <nav
      aria-label={m.launcher_actions()}
      class="flex shrink-0 items-stretch py-4 tall:py-6"
    >
      <div class="relative w-52 shrink-0 lg:w-72">
        {#if home}
          {#each left as entry, index (entry.id)}
            {@render action(entry, index, true)}
          {/each}
        {/if}
      </div>

      <div
        aria-hidden="true"
        class={[
          "relative h-60 w-88 transition duration-260 ease-out tall:h-80",
          "lg:w-96 xl:w-120",
          home ? "opacity-100" : "scale-95 opacity-15",
        ]}
      >
        <canvas
          class="absolute inset-0 size-full"
          {@attach field(signature, { rows: 40 })}
        ></canvas>
      </div>

      <div class="relative w-52 shrink-0 lg:w-72">
        {#if home}
          {#each right as entry, index (entry.id)}
            {@render action(entry, index, false)}
          {/each}
        {/if}
      </div>
    </nav>

    {#if workspace.erdDraft}
      <div
        role="status"
        inert={!home}
        class={[
          "flex w-full max-w-2xl shrink-0 items-center gap-4 bg-base-100 px-4",
          "py-3 hairline transition-opacity duration-260 ease-out",
          home ? "opacity-100" : "opacity-0",
        ]}
      >
        <Icon icon="lucide:file-clock" class="size-5 shrink-0 text-warning" />

        <div class="min-w-0 flex-1">
          <p class="text-sm font-medium">{m.erd_draft_title()}</p>
          <p class="text-xs text-base-content/70">{m.erd_draft_hint()}</p>
        </div>

        <button
          type="button"
          onclick={() => (dropping = true)}
          class="btn btn-ghost btn-sm font-medium"
        >
          {m.discard()}
        </button>

        <button
          type="button"
          onclick={() => workspace.restoreErdDraft()}
          class="btn btn-soft btn-primary btn-sm font-medium"
        >
          {m.erd_draft_restore()}
        </button>
      </div>
    {/if}
  </div>

  {#if launcher.view !== "home"}
    <div class="absolute inset-0 flex items-center justify-center p-6">
      <section
        data-card
        in:rise
        out:leave
        aria-label={title}
        class={[
          "floating lift flex h-full w-full flex-col",
          FRAME[launcher.view],
        ]}
      >
        <header
          class={[
            "flex shrink-0 items-center gap-2 border-b border-base-content/10",
            "px-2 py-2",
          ]}
        >
          <button
            type="button"
            onclick={back}
            aria-keyshortcuts="Escape"
            class="btn btn-ghost btn-sm font-medium"
          >
            <Icon icon="lucide:arrow-left" class="size-4" />
            {m.back()}
          </button>

          <h2 class="min-w-0 flex-1 truncate text-sm font-semibold">
            {title}
          </h2>
        </header>

        <div data-card-body class="flex min-h-0 flex-1 flex-col">
          {#if launcher.view === "new"}
            <div class="min-h-0 flex-1 overflow-y-auto px-4 pt-4">
              {#key seed}
                <NewSession {seed} onsaved={back} />
              {/key}
            </div>
          {:else if launcher.view === "quick"}
            <QuickConnect onhandoff={handoff} />
          {:else}
            <ConnectionList onedit={handoff} onfresh={fresh} />
          {/if}
        </div>
      </section>
    </div>
  {/if}
</div>

{#if dropping}
  <ConfirmDialog
    title={m.erd_draft_discard_ask()}
    confirm={m.discard()}
    cancel={m.cancel()}
    onconfirm={() => {
      dropping = false
      void workspace.discardErdDraft()
    }}
    oncancel={() => (dropping = false)}
  />
{/if}
