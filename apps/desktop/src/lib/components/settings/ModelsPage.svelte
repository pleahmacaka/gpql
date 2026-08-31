<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Dropdown, Icon, OptionRow } from "@gpql/ui"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Provider } from "$lib/types"

  const SHOPS = [
    {
      id: "openrouter",
      name: "OpenRouter",
      icon: "simple-icons:openrouter",
      oauth: true,
    },
    {
      id: "openai",
      name: "OpenAI",
      icon: "simple-icons:openai",
      oauth: false,
    },
  ]

  let note = $state("")
  let adding = $state(false)
  let draft = $state({ name: "", baseUrl: "https://api.openai.com/v1", key: "" })
  let linking = $state("")
  let typing = $state("")
  let key = $state("")
  let catalogue = $state<Record<string, string[]>>({})

  const held = (id: string) =>
    workspace.providers.find(provider => provider.id === id)

  async function connect(id: string) {
    if (id !== "openrouter") {
      typing = id
      key = ""

      return
    }

    linking = id
    note = m.openrouter_waiting()

    try {
      await api.run(api.connectOpenrouter(held(id)?.model ?? ""))
      await workspace.reloadProviders()
      await workspace.pick(id)
      note = ""
    } catch (failure) {
      note = `${m.openrouter_failed()}, ${failure}`
    } finally {
      linking = ""
    }
  }

  async function keep(id: string) {
    if (key.trim() === "") {
      return
    }

    await api.run(
      api.saveProvider({
        id,
        name: SHOPS.find(shop => shop.id === id)?.name ?? id,
        baseUrl: "https://api.openai.com/v1",
        model: "gpt-4o-mini",
        key: key.trim(),
      }),
    )

    key = ""
    typing = ""
    await workspace.reloadProviders()
    await workspace.pick(id)
  }

  async function add() {
    const name = draft.name.trim()

    if (name === "" || draft.key.trim() === "") {
      return
    }

    await api.run(
      api.saveProvider({
        id: name.toLowerCase().replaceAll(" ", "-"),
        name,
        baseUrl: draft.baseUrl.trim(),
        model: "",
        key: draft.key.trim(),
      }),
    )

    draft = { name: "", baseUrl: "https://api.openai.com/v1", key: "" }
    adding = false
    await workspace.reloadProviders()
  }

  async function drop(id: string) {
    await api.run(api.forgetProvider(id))
    await workspace.reloadProviders()

    if (workspace.picked === id) {
      await workspace.pick(workspace.providers[0]?.id ?? "")
    }
  }

  async function pickModel(provider: Provider, model: string) {
    await api.run(api.saveProvider({ ...provider, model }))
    await workspace.reloadProviders()
  }

  async function load(provider: Provider) {
    if (catalogue[provider.id]) {
      return
    }

    if (provider.id === "openrouter") {
      catalogue = {
        ...catalogue,
        [provider.id]: await api.run(api.openrouterModels()),
      }

      return
    }

    const answer = await fetch(`${provider.baseUrl}/models`, {
      headers: { authorization: `Bearer ${provider.key}` },
    })

    const body = (await answer.json()) as { data?: { id: string }[] }

    catalogue = {
      ...catalogue,
      [provider.id]: (body.data ?? []).map(entry => entry.id).sort(),
    }
  }

  let mine = $derived(
    workspace.providers.filter(
      provider => !SHOPS.some(shop => shop.id === provider.id),
    ),
  )

  $effect(() => {
    for (const provider of workspace.providers) {
      void load(provider).catch(() => {})
    }
  })
</script>

<div class="pb-3">
  <OptionRow
    icon="lucide:sparkles"
    title={m.ai_on()}
    detail={m.ai_on_hint()}
    on={workspace.ai}
    onclick={() => workspace.toggle("ai")}
  />
</div>

<div class="space-y-2">
  {#each SHOPS as shop (shop.id)}
    {@const provider = held(shop.id)}
    {@const chosen = workspace.model?.id === shop.id}

    <section class="rounded-field bg-base-200 p-3">
      <div class="flex items-center gap-2">
        <Icon icon={shop.icon} class="size-4 shrink-0" />

        <span class="flex-1 text-sm font-medium">{shop.name}</span>

        {#if provider}
          <span
            class="rounded-selector bg-success/15 px-2 py-1 text-xs
              text-success"
          >
            {m.provider_linked()}
          </span>
        {/if}

        {#if chosen}
          <span
            class="rounded-selector bg-primary/15 px-2 py-1 text-xs
              text-primary"
          >
            {m.provider_chosen()}
          </span>
        {/if}
      </div>

      {#if provider}
        <div class="flex items-center gap-2 pt-3">
          <span class="text-xs text-base-content/45">{m.field_model()}</span>

          <div class="min-w-0 flex-1">
            <Dropdown
              wide
              value={provider.model}
              options={(catalogue[provider.id] ?? [provider.model]).map(
                name => ({ value: name, label: name }),
              )}
              search={m.search_models()}
              empty={m.no_match()}
              onpick={name => pickModel(provider, name)}
            />
          </div>

          {#if !chosen}
            <button
              type="button"
              onclick={() => workspace.pick(shop.id)}
              class="rounded-field bg-base-100 px-2 py-1 text-xs hairline
                hover:bg-base-300"
            >
              {m.provider_use()}
            </button>
          {/if}

          <button
            type="button"
            onclick={() => drop(shop.id)}
            class="rounded-field px-2 py-1 text-xs text-base-content/50
              hover:text-error"
          >
            {m.disconnect()}
          </button>
        </div>
      {:else if typing === shop.id}
        <div class="flex gap-1 pt-3">
          <input
            bind:value={key}
            type="password"
            placeholder={m.field_api_key()}
            onkeydown={event => {
              if (event.key === "Enter") {
                event.preventDefault()
                keep(shop.id)
              }
            }}
            class="min-w-0 flex-1 rounded-field bg-base-100 px-2 py-1 text-sm
              outline-none select-text"
          />

          <button
            type="button"
            onclick={() => keep(shop.id)}
            class="rounded-field bg-primary px-3 py-1 text-sm
              text-primary-content"
          >
            {m.save_credential()}
          </button>
        </div>
      {:else}
        <button
          type="button"
          disabled={linking === shop.id}
          onclick={() => connect(shop.id)}
          class="mt-3 flex w-full items-center justify-center gap-2 rounded-field
            bg-base-100 py-2 text-sm hairline hover:bg-base-300
            disabled:opacity-60"
        >
          <Icon
            icon={shop.oauth ? "lucide:log-in" : "lucide:key-round"}
            class="size-4"
          />
          {shop.oauth ? m.provider_connect() : m.provider_key()}
        </button>
      {/if}
    </section>
  {/each}
</div>

{#each mine as provider (provider.id)}
  {@const chosen = workspace.model?.id === provider.id}

  <section class="mt-2 rounded-field bg-base-200 p-3">
    <div class="flex items-center gap-2">
      <Icon icon="lucide:sparkles" class="size-4 shrink-0 text-accent" />

      <span class="flex-1 truncate text-sm font-medium">{provider.name}</span>

      <span class="truncate font-mono text-xs text-base-content/40">
        {provider.baseUrl}
      </span>

      {#if chosen}
        <span
          class="rounded-selector bg-primary/15 px-2 py-1 text-xs text-primary"
        >
          {m.provider_chosen()}
        </span>
      {/if}
    </div>

    <div class="flex items-center gap-2 pt-3">
      <span class="text-xs text-base-content/45">{m.field_model()}</span>

      <div class="min-w-0 flex-1">
        <Dropdown
          wide
          value={provider.model}
          options={(catalogue[provider.id] ?? [provider.model])
            .filter(Boolean)
            .map(name => ({ value: name, label: name }))}
          search={m.search_models()}
          empty={m.no_match()}
          onpick={name => pickModel(provider, name)}
        />
      </div>

      {#if !chosen}
        <button
          type="button"
          onclick={() => workspace.pick(provider.id)}
          class="rounded-field bg-base-100 px-2 py-1 text-xs hairline
            hover:bg-base-300"
        >
          {m.provider_use()}
        </button>
      {/if}

      <button
        type="button"
        onclick={() => drop(provider.id)}
        class="rounded-field px-2 py-1 text-xs text-base-content/50
          hover:text-error"
      >
        {m.disconnect()}
      </button>
    </div>
  </section>
{/each}

{#if adding}
  <div class="mt-2 space-y-1 rounded-field bg-base-200 p-3">
    <input
      bind:value={draft.name}
      placeholder={m.field_name()}
      class="w-full rounded-field bg-base-100 px-2 py-1 text-sm outline-none
        select-text"
    />

    <input
      bind:value={draft.baseUrl}
      placeholder={m.field_base_url()}
      class="w-full rounded-field bg-base-100 px-2 py-1 font-mono text-xs
        outline-none select-text"
    />

    <input
      bind:value={draft.key}
      type="password"
      placeholder={m.field_api_key()}
      class="w-full rounded-field bg-base-100 px-2 py-1 text-sm outline-none
        select-text"
    />

    <div class="flex gap-1 pt-1">
      <button
        type="button"
        onclick={() => (adding = false)}
        class="flex-1 rounded-field py-2 text-sm hover:bg-base-300"
      >
        {m.cancel()}
      </button>

      <button
        type="button"
        onclick={add}
        class="flex-1 rounded-field bg-primary py-2 text-sm
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
    class="mt-2 flex w-full items-center justify-center gap-2 rounded-field
      border border-dashed border-base-content/15 py-2 text-sm
      text-base-content/50 hover:border-primary/40 hover:text-primary"
  >
    <Icon icon="lucide:plus" class="size-4" />
    {m.provider_add()}
  </button>
{/if}

<section class="mt-4 rounded-field bg-base-200 p-3">
  <div class="flex items-center gap-2 pb-1">
    <Icon icon="lucide:flask-conical" class="size-4 text-base-content/40" />

    <h3 class="flex-1 text-sm font-medium">{m.labs()}</h3>
  </div>

  <OptionRow
    icon="lucide:group"
    title={m.labs_groups()}
    detail={m.labs_groups_hint()}
    on={workspace.aiGroups}
    onclick={() => workspace.toggle("aiGroups")}
  />
</section>

{#if note}
  <p class="px-1 pt-2 text-xs text-base-content/45">{note}</p>
{/if}
