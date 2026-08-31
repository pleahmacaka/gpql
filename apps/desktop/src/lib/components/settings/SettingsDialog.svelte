<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { fade, scale } from "svelte/transition"

  import { Icon, Lazy, pop, veil } from "@gpql/ui"

  import AppearancePage from "./AppearancePage.svelte"
  import AccountPage from "./AccountPage.svelte"
  import CredentialsPage from "./CredentialsPage.svelte"
  import KeysPage from "./KeysPage.svelte"
  import ModelsPage from "./ModelsPage.svelte"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  const pages = [
    {
      id: "look",
      label: m.settings_appearance(),
      icon: "lucide:sun-moon",
    },
    {
      id: "account",
      label: m.settings_account(),
      icon: "lucide:user-round",
    },
    {
      id: "credentials",
      label: m.settings_credentials(),
      icon: "lucide:key-round",
    },
    {
      id: "models",
      label: m.settings_models(),
      icon: "lucide:sparkles",
    },
    {
      id: "keys",
      label: m.settings_keys(),
      icon: "lucide:command",
    },
    {
      id: "about",
      label: m.settings_about(),
      icon: "lucide:info",
    },
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
  aria-label={m.settings()}
  tabindex="-1"
>
  <nav
    class="flex w-48 shrink-0 flex-col gap-1 border-r border-base-content/10
      bg-base-200 p-3"
  >
    <p
      class="px-2 pt-2 pb-3 text-xs font-medium tracking-wide
        text-base-content/40"
    >
      {m.settings()}
    </p>

    {#each pages as entry (entry.id)}
      <button
        type="button"
        onclick={() => (page = entry.id)}
        aria-pressed={page === entry.id}
        class="group relative flex items-center gap-3 rounded-field py-2
          pr-2 pl-3 text-left text-sm transition-colors {page === entry.id
          ? 'bg-base-100 font-medium hairline'
          : 'text-base-content/70 hover:bg-base-300'}"
      >
        {#if page === entry.id}
          <span
            aria-hidden="true"
            class="absolute inset-y-2 left-0 w-1 rounded-selector bg-primary"
          ></span>
        {/if}

        <Icon
          icon={entry.icon}
          class="size-4 shrink-0 {page === entry.id
            ? 'text-primary'
            : 'text-base-content/40 group-hover:text-base-content/70'}"
        />

        <span class="truncate">{entry.label}</span>
      </button>
    {/each}
  </nav>

  <section class="flex min-w-0 flex-1 flex-col bg-base-100">
    <div class="flex justify-end px-3 pt-3">
      <button
        type="button"
        aria-label={m.close()}
        onclick={onclose}
        class="rounded-selector p-2 text-base-content/40 hover:bg-base-200
          hover:text-base-content"
      >
        <Icon icon="lucide:x" class="size-4" />
      </button>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-5 pb-6">
      {#key page}
        <div in:fade={veil()}>
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
      {/key}
    </div>
  </section>
</div>
