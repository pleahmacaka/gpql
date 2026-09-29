<script lang="ts">
  import {
    ConfirmDialog,
    EmptyState,
    Icon,
    leave,
    Marker,
    OptionRow,
    pop,
    rem,
    rise,
    scramble,
    tooltip,
    trap,
  } from "@gpql/ui"
  import { untrack } from "svelte"
  import { fade, scale } from "svelte/transition"

  import * as m from "$lib/paraglide/messages"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = { anchor: HTMLElement; onclose: () => void }

  let { anchor, onclose }: Props = $props()

  const WIDTH = 24

  let busy = $state(false)
  let listing = $state(false)
  let copied = $state<string | null>(null)
  let closing = $state<string | null>(null)
  let spot = $state(measure())

  let room = $derived(workspace.shared)
  let others = $derived(workspace.rooms.filter(entry => entry.id !== room?.id))

  function measure() {
    const box = anchor.getBoundingClientRect()
    const unit = rem(1)
    const width = Math.min(rem(WIDTH), window.innerWidth - rem(2))

    return {
      left: Math.max(rem(1), box.right - width) / unit,
      top: box.bottom / unit + 0.5,
    }
  }

  $effect(() => {
    const place = () => (spot = measure())

    window.addEventListener("resize", place)

    return () => window.removeEventListener("resize", place)
  })

  $effect(() => {
    if (!workspace.signedIn) {
      return
    }

    untrack(async () => {
      listing = true
      await workspace.loadRooms()
      listing = false
    })
  })

  async function guard(work: () => Promise<unknown>) {
    busy = true

    try {
      await work()
    } catch (thrown) {
      workspace.error = String(thrown)
    } finally {
      busy = false
    }
  }

  const publish = () => guard(() => workspace.publish())

  const flip = () => guard(() => workspace.setShareOpen(!room?.open))

  const visit = (link: string) => guard(() => api.run(api.openLink(link)))

  async function copy(link: string) {
    await navigator.clipboard.writeText(link)
    copied = link

    setTimeout(() => {
      if (copied === link) {
        copied = null
      }
    }, 1400)
  }

  function close() {
    const id = closing

    closing = null

    if (id === null) {
      return
    }

    void guard(() =>
      id === room?.id ? workspace.closeShare() : workspace.closeRoom(id),
    )
  }
</script>

{#snippet copyButton(link: string, label: string)}
  <button
    type="button"
    aria-label={label}
    use:tooltip={label}
    onclick={() => copy(link)}
    class="btn btn-square btn-ghost btn-sm"
  >
    {#key copied === link}
      <span in:scale={pop()} class="grid place-items-center">
        <Icon
          icon={copied === link ? "lucide:check" : "lucide:copy"}
          class={["size-4", copied === link && "text-success"]}
        />
      </span>
    {/key}
  </button>
{/snippet}

<button
  type="button"
  tabindex="-1"
  aria-label={m.close()}
  onclick={onclose}
  class="fixed inset-0 z-40 cursor-default"
></button>

<div
  role="dialog"
  aria-label={m.share_erd()}
  tabindex="-1"
  use:trap={onclose}
  in:rise
  out:leave
  style:left="{spot.left}rem"
  style:top="{spot.top}rem"
  style:max-height="calc(100vh - {spot.top + 1}rem)"
  class={[
    "hud hud-lit floating lift fixed z-50 flex w-96 max-w-full flex-col",
    "origin-top-right outline-none",
  ]}
>
  <header
    class={[
      "flex h-12 shrink-0 items-center gap-2 border-b border-base-content/10",
      "pr-2 pl-4",
    ]}
  >
    <Icon icon="lucide:share-2" class="size-4 text-primary" />

    <h2 class="min-w-0 flex-1 truncate text-sm font-semibold">
      {m.share_erd()}
    </h2>

    <button
      type="button"
      aria-label={m.close()}
      onclick={onclose}
      class="btn btn-square btn-ghost btn-sm"
    >
      <Icon icon="lucide:x" class="size-4" />
    </button>
  </header>

  <div class="min-h-0 overflow-y-auto">
    <section class="flex min-h-60 flex-col justify-center gap-4 p-4">
      {#if !workspace.signedIn}
        <EmptyState art="link" title={m.share_needs_login()} class="p-0" />
      {:else if !room}
        <div in:fade={{ duration: 140 }} class="flex flex-col gap-4">
          <EmptyState art="link" title={m.share_hint()} class="p-0" />

          <button
            type="button"
            disabled={busy || !workspace.session}
            onclick={publish}
            class="btn btn-primary btn-sm w-full font-medium"
          >
            {#if busy}
              <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
            {:else}
              <Icon icon="lucide:upload" class="size-4" />
            {/if}

            {busy ? m.share_working() : m.share_publish()}
          </button>
        </div>
      {:else}
        {@const live = room}

        <div in:fade={{ duration: 140 }} class="flex flex-col gap-4">
          <div class="flex flex-col gap-1">
            <Marker label={m.share_link()} />

            <div
              class={[
                "flex items-center gap-1 bg-base-200 py-1 pr-1 pl-3",
                "hairline",
              ]}
            >
              {#key live.link}
                <p
                  use:scramble
                  class="min-w-0 flex-1 truncate text-xs select-text"
                >
                  {live.link}
                </p>
              {/key}

              {@render copyButton(live.link, m.menu_copy())}

              <button
                type="button"
                aria-label={m.share_open_browser()}
                use:tooltip={m.share_open_browser()}
                onclick={() => visit(live.link)}
                class="btn btn-square btn-ghost btn-sm"
              >
                <Icon icon="lucide:external-link" class="size-4" />
              </button>
            </div>
          </div>

          <div class="flex flex-col gap-1">
            <Marker label={m.share_access()} />

            <div class="hairline">
              <OptionRow
                icon={live.open ? "lucide:globe" : "lucide:lock"}
                title={m.share_public()}
                detail={live.open
                  ? m.share_public_hint()
                  : m.share_private_hint()}
                on={live.open}
                disabled={busy}
                onclick={flip}
              />
            </div>
          </div>

          <div class="flex gap-2">
            <button
              type="button"
              disabled={busy}
              onclick={publish}
              class="btn btn-soft btn-sm flex-1 font-medium"
            >
              <Icon
                icon={busy ? "lucide:loader-circle" : "lucide:refresh-cw"}
                class={["size-4", busy && "animate-spin"]}
              />
              {m.share_again()}
            </button>

            <button
              type="button"
              disabled={busy}
              onclick={() => (closing = live.id)}
              class="btn btn-ghost btn-sm font-medium text-error"
            >
              <Icon icon="lucide:link-2-off" class="size-4" />
              {m.share_close()}
            </button>
          </div>
        </div>
      {/if}
    </section>

    {#if workspace.signedIn}
      <section
        class="flex flex-col gap-2 border-t border-base-content/10 p-4"
      >
        <Marker as="h3" label={m.share_rooms()} />

        <div aria-busy={listing} class="h-40 overflow-y-auto hairline">
          {#if listing && others.length === 0}
            <p
              role="status"
              in:fade={{ duration: 140 }}
              class={[
                "flex h-full items-center justify-center gap-2 text-xs",
                "text-base-content/70",
              ]}
            >
              <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
              {m.share_rooms_loading()}
            </p>
          {:else if others.length === 0}
            <p
              in:fade={{ duration: 140 }}
              class={[
                "flex h-full items-center justify-center text-xs",
                "text-base-content/70",
              ]}
            >
              {m.share_rooms_none()}
            </p>
          {:else}
            <ul class="divide-y divide-base-content/10">
              {#each others as entry (entry.id)}
                <li
                  in:fade={{ duration: 140 }}
                  class="flex items-center gap-2 py-1 pr-1 pl-3"
                >
                  <Icon
                    icon={entry.open ? "lucide:globe" : "lucide:lock"}
                    class="size-4 shrink-0 text-base-content/70"
                  />

                  <div class="min-w-0 flex-1">
                    <p class="truncate text-xs font-medium">{entry.name}</p>
                    <p
                      class="truncate text-xs text-base-content/70 select-text"
                    >
                      {entry.link}
                    </p>
                  </div>

                  {@render copyButton(entry.link, m.menu_copy())}

                  <button
                    type="button"
                    aria-label="{m.share_close()}, {entry.name}"
                    use:tooltip={m.share_close()}
                    disabled={busy}
                    onclick={() => (closing = entry.id)}
                    class="btn btn-square btn-ghost btn-sm hover:text-error"
                  >
                    <Icon icon="lucide:link-2-off" class="size-4" />
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </section>
    {/if}
  </div>

  <p class="sr-only" aria-live="polite">{copied ? m.share_copied() : ""}</p>
</div>

{#if closing !== null}
  <ConfirmDialog
    title={m.share_close_ask()}
    body={m.share_close_hint()}
    icon="lucide:link-2-off"
    confirm={m.share_close()}
    cancel={m.cancel()}
    onconfirm={close}
    oncancel={() => (closing = null)}
  />
{/if}
