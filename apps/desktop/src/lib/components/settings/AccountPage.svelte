<script lang="ts">
  import { Icon } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import { site, sync, wipeCloud } from "$lib/sync/client"

  let note = $state("")
  let armed = $state<"local" | "cloud" | null>(null)

  async function signIn() {
    note = m.probe_checking()

    try {
      await api.run(api.signIn(site))
      await workspace.refreshAccount()
      note = m.sign_in()
    } catch (failure) {
      note = String(failure)
    }
  }

  async function syncNow() {
    note = m.probe_checking()

    try {
      note = await sync()
      await workspace.reloadRecents()
      await workspace.query.reload()
    } catch (failure) {
      note = String(failure)
    }
  }

  async function signOut() {
    await api.run(api.forgetAccount())
    await workspace.refreshAccount()
    note = ""
  }

  async function wipe(scope: "local" | "cloud") {
    if (armed !== scope) {
      armed = scope

      return
    }

    armed = null
    note = m.probe_checking()

    try {
      note =
        scope === "local" ? await workspace.wipeLocal() : await wipeCloud()
    } catch (failure) {
      note = String(failure)
    }
  }
</script>

<div class="flex items-center gap-2">
  {#if workspace.signedIn}
    <button
      type="button"
      onclick={syncNow}
      class="btn btn-primary btn-sm h-8 flex-1 rounded-field border-0 font-normal
        shadow-none"
    >
      {m.sync_now()}
    </button>

    <button
      type="button"
      onclick={signOut}
      class="text-sm text-error hover:underline"
    >
      {m.sign_out()}
    </button>
  {:else}
    <button
      type="button"
      onclick={signIn}
      class="btn btn-primary btn-sm h-8 flex-1 rounded-field border-0 font-normal
        shadow-none"
    >
      {m.sign_in()}
    </button>
  {/if}
</div>

<div class="mt-4 space-y-2">
  {#each [{ scope: "local" as const, title: m.reset_local(), hint: m.reset_local_hint(), icon: "lucide:hard-drive" }, { scope: "cloud" as const, title: m.reset_cloud(), hint: m.reset_cloud_hint(), icon: "lucide:cloud-off" }] as entry (entry.scope)}
    <div class="flex items-center gap-3 rounded-field bg-base-200 px-3 py-3">
      <Icon icon={entry.icon} class="size-4 text-base-content/40" />

      <div class="min-w-0 flex-1">
        <p class="text-sm">{entry.title}</p>

        <p class="truncate text-xs text-base-content/45">
          {armed === entry.scope ? m.confirm_again() : entry.hint}
        </p>
      </div>

      <button
        type="button"
        onclick={() => wipe(entry.scope)}
        disabled={entry.scope === "cloud" && !workspace.signedIn}
        class="rounded-field px-3 py-1 text-xs
          {armed === entry.scope
          ? 'bg-error text-error-content'
          : 'text-error hover:bg-base-300'}
          disabled:text-base-content/25 disabled:hover:bg-transparent"
      >
        {m.reset()}
      </button>
    </div>
  {/each}
</div>

{#if note}
  <p class="px-1 pt-3 text-xs text-base-content/45">{note}</p>
{/if}
