<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { onMount } from "svelte"

  import { Icon } from "@gpql/ui"
  import * as api from "$lib/session/commands"
  import type { SavedLogin } from "$lib/types"

  let logins = $state<SavedLogin[]>([])

  onMount(() => {
    reload()
  })

  async function reload() {
    logins = await api.run(api.savedLogins())
  }

  async function drop(url: string) {
    await api.run(api.forgetLogin(url))
    await reload()
  }

  async function dropAll() {
    await api.run(api.forgetAllLogins())
    await reload()
  }
</script>

<div class="flex items-baseline">
  <p class="flex-1 text-xs text-base-content/45">
    {m.logins_note()}
  </p>

  <button
    type="button"
    onclick={dropAll}
    class="text-sm text-error hover:underline"
  >
    {m.forget_all()}
  </button>
</div>

<div
  class="mt-1 max-h-28 space-y-1 overflow-y-auto rounded-field bg-base-200 p-2"
>
  {#each logins as login (login.url)}
    <div class="group flex items-center gap-2">
      <Icon icon="lucide:lock" class="size-3.5 shrink-0 text-base-content/35" />

      <span class="flex-1 truncate text-xs text-base-content/70">
        {login.url}
      </span>

      <button
        type="button"
        aria-label="Forget {login.url}"
        onclick={() => drop(login.url)}
        class="text-base-content/30 opacity-0 group-hover:opacity-100
          hover:text-error focus-visible:opacity-100"
      >
        <Icon icon="lucide:x" class="size-3.5" />
      </button>
    </div>
  {:else}
    <p class="text-xs text-base-content/45">
      Connect once and the login lands here.
    </p>
  {/each}
</div>

<p class="px-1 pt-1.5 text-xs text-base-content/40">
  {m.sealed_here()}
</p>
