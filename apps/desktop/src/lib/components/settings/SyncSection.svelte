<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import { site, sync } from "$lib/sync/client"

  let note = $state("")

  async function signIn() {
    note = "waiting for the browser"

    try {
      await api.run(api.signIn(site))
      await workspace.refreshAccount()
      note = "signed in"
    } catch (failure) {
      note = String(failure)
    }
  }

  async function syncNow() {
    note = "syncing"

    try {
      note = await sync()
      await workspace.reloadRecents()
      await workspace.reloadSaved()
    } catch (failure) {
      note = String(failure)
    }
  }

  async function signOut() {
    await api.run(api.forgetAccount())
    await workspace.refreshAccount()
    note = ""
  }
</script>

<p class="pb-2 text-xs text-base-content/45">
  {m.sync_note()}
</p>

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

{#if note}
  <p class="px-1 pt-2 text-xs text-base-content/45">{note}</p>
{/if}
