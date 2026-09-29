<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Dropdown, Icon, OptionRow, RowGroup } from "@gpql/ui"
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
  let draft = $state({
    name: "",
    baseUrl: "https://api.openai.com/v1",
    key: "",
  })
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
        id: `custom-${crypto.randomUUID()}`,
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

{#snippet model(provider: Provider, chosen: boolean, id: string)}
  <div class="flex items-center gap-2 pt-3">
    <div class="min-w-0 flex-1">
      <Dropdown
        wide
        label={m.field_model()}
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
        onclick={() => workspace.pick(id)}
        class="btn btn-soft btn-sm font-medium"
      >
        {m.provider_use()}
      </button>
    {/if}

    <button
      type="button"
      onclick={() => drop(id)}
      class="btn btn-ghost btn-sm font-medium hover:text-error"
    >
      {m.disconnect()}
    </button>
  </div>
{/snippet}

{#snippet badges(linked: boolean, chosen: boolean)}
  {#if linked}
    <span class="badge badge-sm badge-soft badge-success">
      {m.provider_linked()}
    </span>
  {/if}

  {#if chosen}
    <span class="badge badge-sm badge-soft badge-primary">
      {m.provider_chosen()}
    </span>
  {/if}
{/snippet}

<RowGroup>
  <OptionRow
    icon="lucide:sparkles"
    title={m.ai_on()}
    detail={m.ai_on_hint()}
    on={workspace.ai}
    onclick={() => workspace.toggle("ai")}
  />
</RowGroup>

<RowGroup label={m.settings_providers()}>
  {#each SHOPS as shop (shop.id)}
    {@const provider = held(shop.id)}
    {@const chosen = workspace.model?.id === shop.id}

    <section class="px-4 py-4">
      <div class="flex items-center gap-2">
        <Icon icon={shop.icon} class="size-4 shrink-0" />
        <h4 class="flex-1 text-sm font-medium">{shop.name}</h4>
        {@render badges(!!provider, chosen)}
      </div>

      {#if provider}
        {@render model(provider, chosen, shop.id)}
      {:else if typing === shop.id}
        <div class="flex gap-2 pt-3">
          <input
            bind:value={key}
            type="password"
            placeholder={m.field_api_key()}
            aria-label={m.field_api_key()}
            onkeydown={event => {
              if (event.key === "Enter" && !event.isComposing) {
                event.preventDefault()
                keep(shop.id)
              }
            }}
            class="input input-sm min-w-0 flex-1 bg-base-100 select-text"
          />

          <button
            type="button"
            onclick={() => keep(shop.id)}
            class="btn btn-primary btn-sm font-medium"
          >
            {m.save_credential()}
          </button>
        </div>
      {:else}
        <button
          type="button"
          disabled={linking === shop.id}
          onclick={() => connect(shop.id)}
          class="btn btn-soft btn-sm mt-3 w-full font-medium"
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

  {#each mine as provider (provider.id)}
    {@const chosen = workspace.model?.id === provider.id}

    <section class="px-4 py-4">
      <div class="flex items-center gap-2">
        <Icon icon="lucide:sparkles" class="size-4 shrink-0 text-primary" />

        <h4 class="min-w-0 flex-1 truncate text-sm font-medium">
          {provider.name}
        </h4>

        <span class="max-w-48 truncate text-xs text-base-content/70">
          {provider.baseUrl}
        </span>

        {@render badges(false, chosen)}
      </div>

      {@render model(provider, chosen, provider.id)}
    </section>
  {/each}

  {#if adding}
    <div class="flex flex-col gap-2 bg-base-content/5 p-4">
      <input
        bind:value={draft.name}
        placeholder={m.field_name()}
        aria-label={m.field_name()}
        class="input input-sm w-full bg-base-100 select-text"
      />

      <input
        bind:value={draft.baseUrl}
        placeholder={m.field_base_url()}
        aria-label={m.field_base_url()}
        class="input input-sm w-full bg-base-100 select-text"
      />

      <input
        bind:value={draft.key}
        type="password"
        placeholder={m.field_api_key()}
        aria-label={m.field_api_key()}
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
      {m.provider_add()}
    </button>
  {/if}
</RowGroup>

<RowGroup label={m.labs()}>
  <OptionRow
    icon="lucide:group"
    title={m.labs_groups()}
    detail={m.labs_groups_hint()}
    on={workspace.aiGroups}
    onclick={() => workspace.toggle("aiGroups")}
  />
</RowGroup>

{#if note}
  <p class="text-xs text-base-content/70">{note}</p>
{/if}
