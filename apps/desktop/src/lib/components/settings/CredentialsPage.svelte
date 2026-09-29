<script lang="ts">
  import { onMount } from "svelte"

  import {
    ConfirmDialog,
    Dropdown,
    Icon,
    OptionRow,
    RowGroup,
    SettingRow,
  } from "@gpql/ui"

  import { withoutLogin } from "$lib/components/session/address"
  import * as m from "$lib/paraglide/messages"
  import * as api from "$lib/session/commands"
  import { friendly } from "$lib/session/errors"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { SavedLogin } from "$lib/types"

  let logins = $state<SavedLogin[]>([])
  let failure = $state("")

  // built-in pairs still fill the connect form but are not yours to manage
  let mine = $derived(workspace.presets.filter(preset => !preset.builtin))
  let adding = $state(false)
  let draft = $state({ name: "", user: "", password: "" })
  let wiping = $state(false)

  onMount(() => {
    void reload()
  })

  async function attempt(work: () => Promise<void>) {
    failure = ""

    try {
      await work()
    } catch (problem) {
      failure = friendly(String(problem))
    }
  }

  function reload() {
    return attempt(async () => {
      logins = await api.run(api.savedLogins())
      await workspace.reloadPresets()
    })
  }

  function add() {
    if (draft.name.trim() === "" || draft.user.trim() === "") {
      return
    }

    return attempt(async () => {
      await api.run(api.saveCredential({ ...draft, name: draft.name.trim() }))
      draft = { name: "", user: "", password: "" }
      adding = false
      await workspace.reloadPresets()
    })
  }

  function dropPreset(name: string) {
    return attempt(async () => {
      await api.run(api.forgetCredential(name))
      await workspace.reloadPresets()
    })
  }

  function dropLogin(url: string) {
    return attempt(async () => {
      await api.run(api.forgetLogin(url))
      await reload()
    })
  }

  function dropAllLogins() {
    wiping = false

    return attempt(async () => {
      await api.run(api.forgetAllLogins())
      await reload()
    })
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.isComposing) {
      event.preventDefault()
      void add()
    }
  }
</script>

<RowGroup label={m.settings_opening()}>
  <SettingRow
    icon="lucide:door-open"
    title={m.option_startup()}
    detail={m.option_startup_hint()}
  >
    <Dropdown
      label={m.option_startup()}
      options={[
        { value: "last", label: m.startup_last() },
        { value: "recent", label: m.startup_recent() },
      ]}
      value={workspace.startup}
      onpick={mode => workspace.setStartup(mode)}
    />
  </SettingRow>

  <OptionRow
    icon="lucide:radar"
    title={m.option_scan()}
    detail={m.option_scan_hint()}
    on={workspace.autoscan}
    onclick={() => workspace.toggle("autoscan")}
  />
</RowGroup>

<RowGroup label={m.settings_credentials()} note={m.credentials_note()}>
  {#each mine as preset (preset.name)}
    <div class="group flex items-center gap-4 px-4 py-3">
      <Icon icon="lucide:key-round" class="size-4 shrink-0 text-primary" />

      <div class="min-w-0 flex-1">
        <p class="truncate text-sm">{preset.name}</p>
        <p class="truncate text-xs text-base-content/70">{preset.user}</p>
      </div>

      <button
        type="button"
        aria-label={m.menu_forget({ name: preset.name })}
        onclick={() => dropPreset(preset.name)}
        class={[
          "btn btn-square btn-ghost btn-xs text-base-content/70 opacity-0",
          "group-hover:opacity-100 hover:text-error focus-visible:opacity-100",
        ]}
      >
        <Icon icon="lucide:trash-2" class="size-4" />
      </button>
    </div>
  {/each}

  {#if adding}
    <div class="flex flex-col gap-2 bg-base-content/5 p-4">
      <input
        bind:value={draft.name}
        placeholder={m.field_name()}
        aria-label={m.field_name()}
        onkeydown={keys}
        class="input input-sm w-full bg-base-100 select-text"
      />

      <input
        bind:value={draft.user}
        placeholder={m.field_user()}
        aria-label={m.field_user()}
        onkeydown={keys}
        class="input input-sm w-full bg-base-100 select-text"
      />

      <input
        bind:value={draft.password}
        type="password"
        placeholder={m.field_password()}
        aria-label={m.field_password()}
        onkeydown={keys}
        class="input input-sm w-full bg-base-100 select-text"
      />

      <div class="flex justify-end gap-2 pt-2">
        <button
          type="button"
          onclick={() => (adding = false)}
          class="btn btn-ghost btn-sm font-medium"
        >
          {m.cancel()}
        </button>

        <button
          type="button"
          onclick={add}
          class="btn btn-primary btn-sm font-medium"
        >
          {m.save_credential()}
        </button>
      </div>
    </div>
  {:else}
    <button
      type="button"
      onclick={() => (adding = true)}
      class={[
        "flex cursor-pointer items-center gap-4 px-4 py-3 text-sm",
        "text-base-content/70 transition-colors hover:bg-base-content/5",
        "hover:text-primary",
      ]}
    >
      <Icon icon="lucide:plus" class="size-4" />
      {m.add()}
    </button>
  {/if}
</RowGroup>

<RowGroup label={m.settings_logins()} note={m.logins_note()}>
  {#snippet actions()}
    {#if logins.length > 0}
      <button
        type="button"
        onclick={() => (wiping = true)}
        class="btn btn-ghost btn-xs font-medium text-error"
      >
        {m.forget_all()}
      </button>
    {/if}
  {/snippet}

  {#each logins as login (login.url)}
    <div class="group flex items-center gap-4 px-4 py-3">
      <Icon
        icon={workspace.iconFor(login.kind)}
        class="size-4 shrink-0 text-base-content/70"
      />

      <span class="min-w-0 flex-1">
        <span class="block truncate text-sm">{withoutLogin(login.url)}</span>

        <span class="block truncate text-xs text-base-content/70">
          {login.hasPassword ? m.login_has_password() : m.login_no_password()}
        </span>
      </span>

      <button
        type="button"
        aria-label={m.menu_forget({ name: withoutLogin(login.url) })}
        onclick={() => dropLogin(login.url)}
        class={[
          "btn btn-square btn-ghost btn-xs text-base-content/70 opacity-0",
          "group-hover:opacity-100 hover:text-error focus-visible:opacity-100",
        ]}
      >
        <Icon icon="lucide:trash-2" class="size-4" />
      </button>
    </div>
  {:else}
    <p class="flex items-center gap-4 px-4 py-4 text-sm text-base-content/70">
      <Icon icon="lucide:shield-check" class="size-4" />
      {m.logins_empty()}
    </p>
  {/each}
</RowGroup>

{#if failure}
  <p class="text-xs wrap-anywhere text-error select-text">{failure}</p>
{/if}

{#if wiping}
  <ConfirmDialog
    title={m.forget_all_title()}
    body={m.forget_all_body()}
    confirm={m.forget_all()}
    cancel={m.cancel()}
    onconfirm={dropAllLogins}
    oncancel={() => (wiping = false)}
  />
{/if}
