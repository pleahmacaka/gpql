<script lang="ts">
  import { fade } from "svelte/transition"

  import { Icon, Lazy, Marker, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"

  import AccountPage from "./AccountPage.svelte"
  import AppearancePage from "./AppearancePage.svelte"
  import CredentialsPage from "./CredentialsPage.svelte"
  import KeysPage from "./KeysPage.svelte"
  import LanguagePage from "./LanguagePage.svelte"
  import ModelsPage from "./ModelsPage.svelte"
  import WritesPage from "./WritesPage.svelte"

  type Props = { onclose: () => void; class?: string }

  let { onclose, class: extra = "" }: Props = $props()

  const pages = [
    {
      id: "look",
      label: m.settings_appearance(),
      note: m.settings_appearance_note(),
      icon: "lucide:sun-moon",
    },
    {
      id: "writes",
      label: m.settings_writes(),
      note: m.settings_writes_note(),
      icon: "lucide:pencil-line",
    },
    {
      id: "connections",
      label: m.settings_connections(),
      note: m.settings_connections_note(),
      icon: "lucide:plug",
    },
    {
      id: "models",
      label: m.settings_models(),
      note: m.models_note(),
      icon: "lucide:sparkles",
    },
    {
      id: "lsp",
      label: m.settings_lsp(),
      note: m.lsp_note(),
      icon: "lucide:code",
    },
    {
      id: "keys",
      label: m.settings_keys(),
      note: m.keys_note(),
      icon: "lucide:command",
    },
    {
      id: "account",
      label: m.settings_account(),
      note: m.sync_note(),
      icon: "lucide:user-round",
    },
    {
      id: "about",
      label: m.settings_about(),
      note: m.settings_about_note(),
      icon: "lucide:info",
    },
  ]

  let page = $state("look")

  let current = $derived(pages.find(entry => entry.id === page) ?? pages[0])
</script>

<div class={["flex min-h-0", extra]}>
  <nav
    aria-label={m.settings()}
    class={[
      "flex w-56 shrink-0 flex-col gap-1 overflow-y-auto border-r",
      "border-base-content/10 bg-base-200/60 p-3",
    ]}
  >
    <Marker as="h2" label={m.settings()} class="px-2 pt-2 pb-3" />

    {#each pages as entry (entry.id)}
      <button
        type="button"
        onclick={() => (page = entry.id)}
        aria-current={page === entry.id ? "page" : undefined}
        class={[
          "relative flex cursor-pointer items-center gap-3 py-2 pr-2 pl-4",
          "text-left text-sm transition-colors",
          page === entry.id
            ? "bg-base-100 font-medium hairline"
            : "text-base-content/70 hover:bg-base-content/5",
        ]}
      >
        {#if page === entry.id}
          <span
            aria-hidden="true"
            class="absolute inset-y-0 left-0 w-1 bg-primary"
          ></span>
        {/if}

        <Icon
          icon={entry.icon}
          class={[
            "size-4 shrink-0",
            page === entry.id ? "text-primary" : "text-base-content/70",
          ]}
        />

        <span class="truncate">{entry.label}</span>
      </button>
    {/each}
  </nav>

  <section class="flex min-w-0 flex-1 flex-col">
    <header
      class={[
        "flex items-start gap-4 border-b border-base-content/10 px-6 pt-6",
        "pb-4",
      ]}
    >
      <div class="min-w-0 flex-1">
        <h2 class="text-lg font-semibold tracking-tight">{current.label}</h2>
        <p class="pt-1 text-xs text-pretty text-base-content/70">
          {current.note}
        </p>
      </div>

      <button
        type="button"
        aria-label={m.close()}
        onclick={onclose}
        class="btn btn-square btn-ghost btn-sm"
      >
        <Icon icon="lucide:x" class="size-4" />
      </button>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-6 py-6">
      {#key page}
        <div in:fade={veil()} class="flex flex-col gap-8">
          {#if page === "look"}
            <AppearancePage />
          {:else if page === "writes"}
            <WritesPage />
          {:else if page === "connections"}
            <CredentialsPage />
          {:else if page === "models"}
            <ModelsPage />
          {:else if page === "lsp"}
            <LanguagePage />
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
