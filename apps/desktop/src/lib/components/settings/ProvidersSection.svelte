<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon } from "@gpql/ui"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"

  let adding = $state(false)
  let note = $state("")
  let draft = $state({
    name: "",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    key: "",
  })

  async function add() {
    if (draft.name.trim() === "" || draft.key.trim() === "") {
      return
    }

    await api.run(
      api.saveProvider({
        ...draft,
        id: draft.name.trim().toLowerCase().replaceAll(" ", "-"),
        name: draft.name.trim(),
      }),
    )

    draft = {
      name: "",
      baseUrl: "https://api.openai.com/v1",
      model: "gpt-4o-mini",
      key: "",
    }
    adding = false
    await workspace.reloadProviders()
  }

  async function drop(id: string) {
    await api.run(api.forgetProvider(id))
    await workspace.reloadProviders()
  }

  async function connectAgent() {
    note = "starting the agent"

    try {
      await workspace.startAgent(workspace.agentCommand)
      note = workspace.agent ? "agent is answering" : "agent stopped"
    } catch (failure) {
      note = String(failure)
    }
  }

  async function openrouter() {
    note = "waiting for the browser"

    try {
      await api.run(api.connectOpenrouter(""))
      await workspace.reloadProviders()
      note = "OpenRouter connected"
    } catch (failure) {
      note = String(failure)
    }
  }
</script>

<div class="flex items-baseline">
  <p class="flex-1 text-xs text-base-content/45">
    {m.models_note()}
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
      bind:value={draft.baseUrl}
      placeholder={m.field_base_url()}
      class="w-full rounded-field bg-base-100 px-2 py-1 text-sm outline-none"
    />
    <input
      bind:value={draft.model}
      placeholder={m.field_model()}
      class="w-full rounded-field bg-base-100 px-2 py-1 text-sm outline-none"
    />
    <input
      bind:value={draft.key}
      type="password"
      placeholder={m.field_api_key()}
      class="w-full rounded-field bg-base-100 px-2 py-1 text-sm outline-none"
    />

    <button
      type="button"
      onclick={add}
      class="w-full rounded-field bg-primary py-1.5 text-sm text-primary-content"
    >
      {m.save_model()}
    </button>
  </div>
{/if}

<div class="mt-1 space-y-1 rounded-field bg-base-200 p-2">
  {#each workspace.providers as provider (provider.id)}
    <div class="group flex items-center gap-2">
      <Icon icon="lucide:sparkles" class="size-3.5 shrink-0 text-accent" />

      <span class="flex-1 truncate text-xs">
        {provider.name}
        <span class="text-base-content/40">· {provider.model}</span>
      </span>

      <button
        type="button"
        aria-label="Forget {provider.name}"
        onclick={() => drop(provider.id)}
        class="text-base-content/30 opacity-0 group-hover:opacity-100
          hover:text-error focus-visible:opacity-100"
      >
        <Icon icon="lucide:x" class="size-3.5" />
      </button>
    </div>
  {:else}
    <p class="text-xs text-base-content/45">
      No model yet. Add an OpenAI-compatible endpoint or sign in with
      OpenRouter.
    </p>
  {/each}
</div>

<div class="mt-3 rounded-field bg-base-200 p-2">
  <div class="flex items-center gap-2 pb-1">
    <Icon
      icon="lucide:circle-dot"
      class="size-3 {workspace.agent ? 'text-success' : 'text-base-content/25'}"
    />

    <span class="flex-1 text-sm">{m.agent()}</span>

    <span class="text-xs text-base-content/40">{m.agent_transport()}</span>
  </div>

  <div class="flex gap-1">
    <input
      value={workspace.agentCommand}
      oninput={event => (workspace.agentCommand = event.currentTarget.value)}
      placeholder="npx @zed-industries/claude-code-acp"
      class="min-w-0 flex-1 rounded-field bg-base-100 px-2 py-1 text-xs
        outline-none placeholder:text-base-content/30"
    />

    <button
      type="button"
      onclick={connectAgent}
      class="rounded-field bg-base-100 px-2 py-1 text-xs hover:bg-base-300"
    >
      {m.apply()}
    </button>
  </div>
</div>

<button
  type="button"
  onclick={openrouter}
  class="mt-2 flex w-full items-center justify-center gap-2 rounded-field
    bg-base-200 py-2 text-sm hover:bg-base-300"
>
  <Icon icon="lucide:key-round" class="size-3.5" />
  {m.connect_openrouter()}
</button>

{#if note}
  <p class="px-1 pt-2 text-xs text-base-content/45">{note}</p>
{/if}
