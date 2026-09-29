<script lang="ts">
  import { onMount } from "svelte"

  import {
    type Ending,
    Icon,
    ListRow,
    leave,
    Marker,
    rem,
    rise,
    trap,
  } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import type { Connection } from "$lib/session/connection.svelte"
  import { workspace } from "$lib/session/workspace.svelte"

  import { withoutLogin } from "./address"
  import { launcher } from "./launcher.svelte"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  let shaking = $state<string | null>(null)
  let ending = $state<Ending>("cancel")

  function done() {
    ending = "confirm"
    onclose()
  }

  let spot = $state({ left: 5, top: 2.5 })

  onMount(() => {
    const chip = document.querySelector("[data-session-chip]")

    if (!chip) {
      return
    }

    const box = chip.getBoundingClientRect()
    const unit = rem(1)

    spot = { left: box.left / unit, top: box.bottom / unit + 0.25 }
  })

  function reason(url: string) {
    const code = workspace.unreachable[url]

    if (code === "gone") {
      return m.file_gone()
    }

    if (code === "refused") {
      return m.bad_credentials()
    }

    if (code === "forgotten") {
      return m.login_forgotten()
    }

    return m.cannot_connect()
  }

  let others = $derived(
    workspace.recents.filter(
      entry => !workspace.connections.some(open => open.origin === entry.url),
    ),
  )

  async function resume(url: string, kind: string) {
    if (workspace.unreachable[url]) {
      shaking = url

      setTimeout(() => {
        if (shaking === url) {
          shaking = null
        }
      }, 400)

      return
    }

    await workspace.resume(url, kind)

    if (!workspace.unreachable[url]) {
      done()
    }
  }

  function jump(id: string) {
    workspace.show(id)
    done()
  }

  function fresh() {
    done()
    launcher.open("new")
  }

  async function shut() {
    const entry = workspace.active

    done()

    if (entry) {
      await workspace.requestClose(entry.id)
    }
  }

  function detail(entry: Connection) {
    return entry.openTransaction
      ? m.tx_open()
      : withoutLogin(entry.handle.detail)
  }
</script>

<button
  type="button"
  tabindex="-1"
  aria-label={m.close()}
  onclick={onclose}
  class="fixed inset-0 z-40 cursor-default"
></button>

<div
  in:rise
  out:leave={{ as: ending }}
  use:trap={onclose}
  role="dialog"
  aria-label={m.session_switch()}
  tabindex="-1"
  class={[
    "floating lift hud hud-lit fixed z-40 flex max-h-4/5 w-md flex-col",
    "outline-none",
  ]}
  style:left="{spot.left}rem"
  style:top="{spot.top}rem"
>
  {#if workspace.connections.length > 0}
    <section class="flex flex-col pb-2">
      <Marker as="h2" label={m.open_here()} class="px-4 pt-4 pb-2" />

      {#each workspace.connections as entry (entry.id)}
        <ListRow
          icon={workspace.iconFor(entry.handle.kind)}
          title={entry.label}
          detail={detail(entry)}
          active={entry.id === workspace.activeId}
          trailing={null}
          dismissLabel={m.session_close({ name: entry.label })}
          onclick={() => jump(entry.id)}
          ondismiss={() => workspace.requestClose(entry.id)}
        />
      {/each}
    </section>
  {/if}

  <section
    class={[
      "flex min-h-0 flex-col overflow-y-auto pb-2",
      workspace.connections.length > 0 && "border-t border-base-content/10",
    ]}
  >
    <Marker
      as="h2"
      tone="muted"
      label={m.opened_before()}
      class="px-4 pt-4 pb-2"
    />

    {#each others as entry (entry.url)}
      <ListRow
        icon={entry.kind === "erd"
          ? "lucide:git-fork"
          : workspace.iconFor(entry.kind)}
        title={entry.alias ?? entry.label}
        detail={workspace.dialing === entry.url
          ? m.connecting_now()
          : workspace.unreachable[entry.url]
            ? reason(entry.url)
            : withoutLogin(entry.detail)}
        tone={workspace.unreachable[entry.url] &&
        workspace.dialing !== entry.url
          ? "bad"
          : "plain"}
        busy={workspace.dialing === entry.url}
        shaking={shaking === entry.url}
        onclick={() => resume(entry.url, entry.kind)}
      />
    {:else}
      <p class="px-4 py-4 text-sm text-base-content/70">
        {m.recent_empty()}
      </p>
    {/each}
  </section>

  <footer
    class="flex items-center gap-2 border-t border-base-content/10 p-2"
  >
    {#if workspace.session}
      <button
        type="button"
        onclick={shut}
        class="btn btn-ghost btn-sm font-medium"
      >
        <Icon icon="lucide:power" class="size-4" />
        {m.action_close()}
      </button>
    {/if}

    <span class="flex-1"></span>

    <button
      type="button"
      onclick={fresh}
      class="btn btn-primary btn-sm font-medium"
    >
      <Icon icon="lucide:plus" class="size-4" />
      {m.action_new()}
    </button>
  </footer>
</div>
