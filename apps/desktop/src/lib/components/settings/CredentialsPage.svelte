<script lang="ts">
  import { onMount } from "svelte"

  import { Icon, OptionRow } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { SavedLogin } from "$lib/types"

  let logins = $state<SavedLogin[]>([])

  // the built-in pairs still fill the connect form; they are not yours to manage
  let mine = $derived(workspace.presets.filter(preset => !preset.builtin))
  let adding = $state(false)
  let draft = $state({ name: "", user: "", password: "" })

  onMount(() => {
    reload()
  })

  async function reload() {
    logins = await api.run(api.savedLogins())
    await workspace.reloadPresets()
  }

  async function add() {
    if (draft.name.trim() === "" || draft.user.trim() === "") {
      return
    }

    await api.run(api.saveCredential({ ...draft, name: draft.name.trim() }))
    draft = { name: "", user: "", password: "" }
    adding = false
    await workspace.reloadPresets()
  }

  async function dropPreset(name: string) {
    await api.run(api.forgetCredential(name))
    await workspace.reloadPresets()
  }

  async function dropLogin(url: string) {
    await api.run(api.forgetLogin(url))
    await reload()
  }

  async function dropAllLogins() {
    await api.run(api.forgetAllLogins())
    await reload()
  }

  function kindOf(login: SavedLogin) {
    return workspace.iconFor(login.kind)
  }
</script>

<section class="space-y-2">
  <OptionRow
    icon="lucide:radar"
    title={m.option_scan()}
    detail={m.option_scan_hint()}
    on={workspace.autoscan}
    onclick={() => workspace.toggle("autoscan")}
  />
</section>

<section class="pt-6">
  <header class="flex items-center gap-2 pb-2">
    <Icon icon="lucide:key-round" class="size-4 text-accent" />

    <h3 class="flex-1 text-sm font-medium">{m.settings_credentials()}</h3>

    <span class="rounded-selector bg-base-200 px-2 py-0.5 text-xs
      text-base-content/50">
      {mine.length}
    </span>
  </header>

  <p class="pb-3 text-xs text-base-content/45">{m.credentials_note()}</p>

  <div class="space-y-1">
    {#each mine as preset (preset.name)}
      <div
        class="group flex items-center gap-3 rounded-field bg-base-200 px-3 py-2"
      >
        <span
          class="grid size-7 shrink-0 place-items-center rounded-selector
            bg-base-100 text-xs font-medium text-base-content/60"
        >
          {preset.name.slice(0, 1).toUpperCase()}
        </span>

        <div class="min-w-0 flex-1">
          <p class="truncate text-sm">{preset.name}</p>

          <p class="truncate text-xs text-base-content/45">
            {preset.user}
            {#if preset.builtin}
             , {m.credentials_typed()}
            {/if}
          </p>
        </div>

        {#if !preset.builtin}
          <button
            type="button"
            aria-label="{m.forget_all()} {preset.name}"
            onclick={() => dropPreset(preset.name)}
            class="rounded-selector p-1 text-base-content/30 opacity-0
              group-hover:opacity-100 hover:text-error focus-visible:opacity-100"
          >
            <Icon icon="lucide:trash-2" class="size-3.5" />
          </button>
        {/if}
      </div>
    {/each}

    {#if adding}
      <div class="space-y-1 rounded-field bg-base-200 p-2">
        <input
          bind:value={draft.name}
          placeholder={m.field_name()}
          class="w-full rounded-field bg-base-100 px-2 py-1.5 text-sm outline-none"
        />
        <input
          bind:value={draft.user}
          placeholder={m.field_user()}
          class="w-full rounded-field bg-base-100 px-2 py-1.5 text-sm outline-none"
        />
        <input
          bind:value={draft.password}
          type="password"
          placeholder={m.field_password()}
          class="w-full rounded-field bg-base-100 px-2 py-1.5 text-sm outline-none"
        />

        <div class="flex gap-1 pt-1">
          <button
            type="button"
            onclick={() => (adding = false)}
            class="flex-1 rounded-field py-1.5 text-sm hover:bg-base-300"
          >
            {m.cancel()}
          </button>

          <button
            type="button"
            onclick={add}
            class="flex-1 rounded-field bg-primary py-1.5 text-sm
              text-primary-content"
          >
            {m.save_credential()}
          </button>
        </div>
      </div>
    {:else}
      <button
        type="button"
        onclick={() => (adding = true)}
        class="flex w-full items-center justify-center gap-1.5 rounded-field
          border border-dashed border-base-content/15 py-2 text-sm
          text-base-content/50 hover:border-primary/40 hover:text-primary"
      >
        <Icon icon="lucide:plus" class="size-3.5" />
        {m.add()}
      </button>
    {/if}
  </div>
</section>

<section class="pt-6">
  <header class="flex items-center gap-2 pb-2">
    <Icon icon="lucide:lock" class="size-4 text-base-content/40" />

    <h3 class="flex-1 text-sm font-medium">{m.settings_logins()}</h3>

    {#if logins.length > 0}
      <button
        type="button"
        onclick={dropAllLogins}
        class="text-xs text-base-content/45 hover:text-error"
      >
        {m.forget_all()}
      </button>
    {/if}
  </header>

  <p class="pb-3 text-xs text-base-content/45">{m.logins_note()}</p>

  <div class="space-y-1">
    {#each logins as login (login.url)}
      <div
        class="group flex items-center gap-3 rounded-field bg-base-200 px-3 py-2"
      >
        <Icon
          icon={kindOf(login)}
          class="size-4 shrink-0 stroke-current stroke-1 opacity-70"
        />

        <span class="min-w-0 flex-1 truncate font-mono text-xs">
          {login.url}
        </span>

        <button
          type="button"
          aria-label="{m.forget_all()} {login.url}"
          onclick={() => dropLogin(login.url)}
          class="rounded-selector p-1 text-base-content/30 opacity-0
            group-hover:opacity-100 hover:text-error focus-visible:opacity-100"
        >
          <Icon icon="lucide:trash-2" class="size-3.5" />
        </button>
      </div>
    {:else}
      <p
        class="rounded-field border border-dashed border-base-content/12 px-3
          py-6 text-center text-xs text-base-content/40"
      >
        {m.logins_empty()}
      </p>
    {/each}
  </div>
</section>
