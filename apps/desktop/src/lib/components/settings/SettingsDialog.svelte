<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  import AppearanceSection from "./AppearanceSection.svelte"
  import ConnectionsSection from "./ConnectionsSection.svelte"
  import CredentialsSection from "./CredentialsSection.svelte"
  import EditorSection from "./EditorSection.svelte"
  import LoginsSection from "./LoginsSection.svelte"
  import ProvidersSection from "./ProvidersSection.svelte"
  import SyncSection from "./SyncSection.svelte"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  const pages = [
    { id: "look", label: m.settings_appearance(), icon: "lucide:sun-moon" },
    { id: "connections", label: m.settings_connections(), icon: "lucide:radar" },
    { id: "credentials", label: m.settings_credentials(), icon: "lucide:key-round" },
    { id: "logins", label: m.settings_logins(), icon: "lucide:lock" },
    { id: "editor", label: m.settings_editor(), icon: "lucide:terminal" },
    { id: "models", label: m.settings_models(), icon: "lucide:sparkles" },
    { id: "sync", label: m.settings_sync(), icon: "lucide:refresh-cw" },
  ]

  let page = $state("look")
</script>

<div
  class="fixed inset-0 z-50 bg-neutral/30"
  role="presentation"
  onclick={onclose}
></div>

<div
  class="fixed inset-x-0 top-12 bottom-12 z-50 mx-auto flex w-2xl max-w-11/12
    overflow-hidden rounded-box bg-base-100 lift"
  role="dialog"
  aria-label="Settings"
  tabindex="-1"
>
  <nav class="flex w-44 shrink-0 flex-col gap-0.5 bg-base-200/60 p-2">
    <p class="px-2 pt-1 pb-2 text-xs text-base-content/45">{m.settings()}</p>

    {#each pages as entry (entry.id)}
      <button
        type="button"
        onclick={() => (page = entry.id)}
        aria-pressed={page === entry.id}
        class="flex items-center gap-2 rounded-field px-2 py-1.5 text-left text-sm
          transition-colors {page === entry.id
          ? 'bg-primary/10 text-primary'
          : 'text-base-content/70 hover:bg-base-300/60'}"
      >
        <Icon icon={entry.icon} class="size-3.5 shrink-0" />
        {entry.label}
      </button>
    {/each}

    <span class="flex-1"></span>

    <p class="truncate px-2 pb-1 text-xs text-base-content/35">
      {workspace.session?.label ?? m.no_session()}
    </p>
  </nav>

  <section class="flex min-w-0 flex-1 flex-col">
    <header class="flex items-center px-5 py-3">
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

    <div class="min-h-0 flex-1 overflow-y-auto px-5 pb-5">
      {#if page === "look"}
        <AppearanceSection />
      {:else if page === "connections"}
        <ConnectionsSection />
      {:else if page === "credentials"}
        <CredentialsSection />
      {:else if page === "logins"}
        <LoginsSection />
      {:else if page === "editor"}
        <EditorSection />
      {:else if page === "models"}
        <ProvidersSection />
      {:else}
        <SyncSection />
      {/if}
    </div>
  </section>
</div>
