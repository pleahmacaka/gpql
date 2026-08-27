<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { fade, scale } from "svelte/transition"

  import { Icon, pop, veil } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  import { Lazy } from "@gpql/ui"

  import AppearancePage from "./AppearancePage.svelte"
  import AccountPage from "./AccountPage.svelte"
  import CredentialsPage from "./CredentialsPage.svelte"
  import KeysPage from "./KeysPage.svelte"
  import ModelsPage from "./ModelsPage.svelte"
    import { loadIcons } from "@iconify/svelte/dist/functions";

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  const pages = [
    { id: "look", label: m.settings_appearance(), icon: "lucide:sun-moon" },
    { id: "account", label: m.settings_account(), icon: "lucide:user-round" },
    {
      id: "credentials",
      label: m.settings_credentials(),
      icon: "lucide:key-round",
    },
    { id: "models", label: m.settings_models(), icon: "lucide:sparkles" },
    { id: "keys", label: m.settings_keys(), icon: "lucide:command" },
    { id: "about", label: m.settings_about(), icon: "lucide:info" },
  ]

  let page = $state("look")
</script>

<div
  transition:fade={veil()}
  class="fixed inset-0 z-50 scrim"
  role="presentation"
  onclick={onclose}
></div>

<div
  transition:scale={pop()}
  class="fixed inset-x-0 top-16 z-50 mx-auto flex h-3/4 w-2xl
    max-w-11/12 overflow-hidden rounded-box floating lift"
  role="dialog"
  aria-label="Settings"
  tabindex="-1"
>
  <nav
    class="flex w-44 shrink-0 flex-col gap-0.5 border-r border-base-content/10
      bg-base-200 p-3"
  >
    <p class="px-2 pt-2 pb-3 text-xs text-base-content/45">{m.settings()}</p>

    {#each pages as entry (entry.id)}
      <button
        type="button"
        onclick={() => (page = entry.id)}
        aria-pressed={page === entry.id}
        class="flex items-center gap-2 rounded-field px-2 py-1.5 text-left text-sm
          transition-colors {page === entry.id
          ? 'bg-base-100 text-primary hairline'
          : 'text-base-content/70 hover:bg-base-300'}"
      >
        <Icon icon={entry.icon} class="size-3.5 shrink-0" />
        {entry.label}
      </button>
    {/each}

  </nav>

  <section class="flex min-w-0 flex-1 flex-col bg-base-100">
    <header
      class="flex items-center border-b border-base-content/10 px-5 py-4"
    >
      <h2 class="flex-1 text-sm font-medium">
        {pages.find(entry => entry.id === page)?.label}
      </h2>

      <button
        type="button"
        aria-label="Close settings"
        onclick={onclose}
        class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
      >
        <Icon icon="lucide:x" class="size-4" />
      </button>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-5 pt-4 pb-6">
      {#if page === "look"}
        <AppearancePage />
      {:else if page === "credentials"}
        <CredentialsPage />
      {:else if page === "models"}
        <ModelsPage />
      {:else if page === "keys"}
        <KeysPage />
      {:else if page === "about"}
        <Lazy load={() => import("./AboutPage.svelte")} />
      {:else}
        <AccountPage />
      {/if}
    </div>
  </section>
</div>
