<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon } from "@gpql/ui"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"

  let adding = $state(false)
  let draft = $state({ name: "", user: "", password: "" })

  async function add() {
    if (draft.name.trim() === "" || draft.user.trim() === "") {
      return
    }

    await api.run(api.saveCredential({ ...draft, name: draft.name.trim() }))
    draft = { name: "", user: "", password: "" }
    adding = false
    await workspace.reloadPresets()
  }

  async function drop(name: string) {
    await api.run(api.forgetCredential(name))
    await workspace.reloadPresets()
  }
</script>

<div class="flex items-baseline">
  <p class="flex-1 text-xs text-base-content/45">
    {m.credentials_note()}
  </p>

  <button
    type="button"
    onclick={() => (adding = !adding)}
    class="text-sm text-primary hover:underline"
  >
    {adding ? m.cancel() : m.add()}
  </button>
</div>

{#if adding}
  <div class="mt-1 space-y-1 rounded-field bg-base-200 p-2">
    <input
      bind:value={draft.name}
      placeholder={m.field_name()}
      class="w-full rounded-field bg-base-100 px-2 py-1 text-sm outline-none"
    />
    <input
      bind:value={draft.user}
      placeholder={m.field_user()}
      class="w-full rounded-field bg-base-100 px-2 py-1 text-sm outline-none"
    />
    <input
      bind:value={draft.password}
      type="password"
      placeholder={m.field_password()}
      class="w-full rounded-field bg-base-100 px-2 py-1 text-sm outline-none"
    />

    <button
      type="button"
      onclick={add}
      class="w-full rounded-field bg-primary py-1.5 text-sm text-primary-content"
    >
      {m.save_credential()}
    </button>
  </div>
{/if}

<div class="mt-1 space-y-1 rounded-field bg-base-200 p-2">
  {#each workspace.presets as preset (preset.name)}
    <div class="group flex items-center gap-2">
      <Icon
        icon="lucide:key-round"
        class="size-3.5 shrink-0 text-base-content/35"
      />

      <span class="flex-1 truncate text-xs">
        {preset.name}
        <span class="text-base-content/40">· {preset.user}</span>
      </span>

      {#if !preset.builtin}
        <button
          type="button"
          aria-label="Forget {preset.name}"
          onclick={() => drop(preset.name)}
          class="text-base-content/30 opacity-0 group-hover:opacity-100
            hover:text-error focus-visible:opacity-100"
        >
          <Icon icon="lucide:x" class="size-3.5" />
        </button>
      {/if}
    </div>
  {/each}
</div>
