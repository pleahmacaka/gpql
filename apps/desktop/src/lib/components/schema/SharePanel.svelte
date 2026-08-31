<script lang="ts">
  import { scale } from "svelte/transition"

  import { Icon, pop } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import { site } from "$lib/sync/client"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  let busy = $state(false)
  let copied = $state(false)
  let failure = $state("")

  let room = $derived(workspace.shared)

  async function guard(work: () => Promise<void>) {
    busy = true
    failure = ""

    try {
      await work()
    } catch (thrown) {
      failure = String(thrown).replace(/^Error:\s*/, "")
    } finally {
      busy = false
    }
  }

  const publish = () => guard(() => workspace.publish(site))

  const flip = () =>
    guard(() => workspace.setShareOpen(site, !workspace.shared?.open))

  const visit = () => guard(() => api.run(api.openLink(room?.link ?? "")))

  async function copy() {
    if (!room) {
      return
    }

    await navigator.clipboard.writeText(room.link)
    copied = true

    setTimeout(() => (copied = false), 1400)
  }
</script>

<div
  class="fixed inset-0 z-30"
  role="presentation"
  onclick={onclose}
  onkeydown={event => event.key === "Escape" && onclose()}
></div>

<div
  transition:scale={pop()}
  class="absolute top-10 right-4 z-40 w-80 space-y-3 rounded-box floating p-3
    lift"
  role="dialog"
  aria-label={m.share_erd()}
  tabindex="-1"
>
  <h2 class="px-1 text-sm font-medium">{m.share_erd()}</h2>

  {#if !workspace.signedIn}
    <p class="px-1 text-xs text-base-content/45">{m.share_needs_login()}</p>
  {:else if !room}
    <p class="px-1 text-xs text-base-content/45">{m.share_hint()}</p>

    <button
      type="button"
      disabled={busy || !workspace.session}
      onclick={publish}
      class="btn btn-primary btn-sm h-9 w-full rounded-field border-0
        font-normal shadow-none"
    >
      {busy ? m.share_working() : m.share_publish()}
    </button>
  {:else}
    <div class="flex items-center gap-2 rounded-field bg-base-200 px-3 py-2">
      <span class="min-w-0 flex-1 truncate font-mono text-xs">{room.link}</span>

      <button
        type="button"
        aria-label={m.menu_copy()}
        onclick={copy}
        class="rounded-selector p-1 text-base-content/50 hover:text-base-content"
      >
        <Icon icon={copied ? "lucide:check" : "lucide:copy"} class="size-4" />
      </button>

      <button
        type="button"
        aria-label={m.share_open_browser()}
        onclick={visit}
        class="rounded-selector p-1 text-base-content/50 hover:text-base-content"
      >
        <Icon icon="lucide:external-link" class="size-4" />
      </button>
    </div>

    <div class="flex items-center gap-3 rounded-field bg-base-200 px-3 py-3">
      <Icon
        icon={room.open ? "lucide:globe" : "lucide:lock"}
        class="size-4 text-accent"
      />

      <div class="min-w-0 flex-1">
        <p class="text-sm">{room.open ? m.share_public() : m.share_private()}</p>

        <p class="text-xs text-base-content/45">
          {room.open ? m.share_public_hint() : m.share_private_hint()}
        </p>
      </div>

      <input
        type="checkbox"
        aria-label={m.share_public()}
        class="toggle toggle-primary toggle-sm"
        checked={room.open}
        disabled={busy}
        onchange={flip}
      />
    </div>

    <button
      type="button"
      disabled={busy}
      onclick={publish}
      class="w-full rounded-field bg-base-200 px-3 py-2 text-xs
        hover:bg-base-300 disabled:opacity-40"
    >
      {busy ? m.share_working() : m.share_again()}
    </button>
  {/if}

  {#if failure}
    <p class="px-1 text-xs text-error">{failure}</p>
  {/if}
</div>
