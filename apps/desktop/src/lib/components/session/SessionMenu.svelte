<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { scale } from "svelte/transition"

  import { Icon, pop } from "@gpql/ui"
  import { ListRow as ListRow } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  let shaking = $state<string | null>(null)

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

  async function resume(url: string) {
    if (workspace.unreachable[url]) {
      shaking = url

      setTimeout(() => {
        if (shaking === url) {
          shaking = null
        }
      }, 400)

      return
    }

    await workspace.resume(url)

    if (!workspace.unreachable[url]) {
      onclose()
    }
  }

  async function fresh() {
    onclose()
    workspace.mode = "new"
    workspace.connecting = true

    if (!workspace.session) {
      await workspace.close()
    }
  }

  async function leave() {
    onclose()
    await workspace.close()
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
  class="absolute top-10 left-16 z-40 w-96 rounded-box floating p-3 lift"
  role="menu"
  tabindex="-1"
>
  <h2 class="px-1 pb-1 text-xs text-base-content/45">{m.opened_before()}</h2>

  {#if workspace.recents.length === 0}
    <p class="px-1 py-3 text-center text-sm text-base-content/45">
      {m.recent_empty()}
    </p>
  {:else}
    {#each workspace.recents as entry (entry.url)}
      <ListRow
        icon={workspace.iconFor(entry.kind)}
        title={entry.label}
        detail={workspace.dialing === entry.url
          ? m.connecting_now()
          : workspace.unreachable[entry.url]
            ? reason(entry.url)
            : entry.detail}
        tone={workspace.unreachable[entry.url] && workspace.dialing !== entry.url
          ? "bad"
          : "plain"}
        busy={workspace.dialing === entry.url}
        shaking={shaking === entry.url}
        onclick={() => resume(entry.url)}
        ondismiss={() => workspace.forgetRecent(entry.url)}
      />
    {/each}
  {/if}

  <div class="mt-2 space-y-1 border-t border-base-content/8 pt-2">
    {#if workspace.session}
      <button
        type="button"
        onclick={leave}
        class="flex w-full items-center gap-2 rounded-field px-3 py-2 text-sm
          hover:bg-base-200"
      >
        <Icon icon="lucide:power" class="size-4 text-base-content/50" />
        {m.action_close()}
      </button>
    {/if}

    <button
      type="button"
      onclick={fresh}
      class="flex w-full items-center gap-2 rounded-field px-3 py-2 text-sm
        text-primary hover:bg-base-200"
    >
      <Icon icon="lucide:plus" class="size-4" />
      New session
    </button>
  </div>
</div>
