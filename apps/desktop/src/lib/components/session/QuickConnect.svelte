<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { onMount } from "svelte"

  import { Icon } from "@gpql/ui"
  import { ListRow as ListRow } from "@gpql/ui"
  import { blankConfig } from "$lib/session/commands"
  import type { Discovery, SessionConfig } from "$lib/types"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = { onhandoff: (config: SessionConfig) => void }

  let { onhandoff }: Props = $props()

  onMount(() => {
    if (workspace.autoscan && workspace.found.length === 0) {
      workspace.scan()
    }
  })

  function toConfig(entry: Discovery): SessionConfig {
    return {
      ...blankConfig(),
      host: entry.host,
      port: entry.port,
      user: entry.user,
      password: entry.password,
      database: entry.database,
    }
  }

  function pick(entry: Discovery) {
    if (entry.needsLogin) {
      onhandoff(toConfig(entry))

      return
    }

    workspace.open(toConfig(entry))
  }
</script>

<div class="space-y-1">
  {#if workspace.scanning}
    <p
      class="flex items-center justify-center gap-2 px-3 py-6 text-sm
        text-base-content/45"
    >
      <span class="loading loading-spinner loading-xs"></span>
      {m.scan_running()}
    </p>
  {:else if workspace.found.length === 0}
    <p class="px-3 py-6 text-center text-sm text-base-content/45">
      {m.scan_empty()}
    </p>
  {:else}
    {#each workspace.found as entry (entry.host + entry.port + entry.database)}
      <ListRow
        icon={entry.needsLogin ? "lucide:lock" : "lucide:database"}
        title={entry.needsLogin
          ? `postgres on ${entry.port}`
          : entry.database}
        detail={entry.needsLogin
          ? m.scan_needs_login()
          : `${entry.user}@${entry.host}:${entry.port}`}
        trailing={entry.needsLogin ? "lucide:pencil" : "lucide:arrow-right"}
        onclick={() => pick(entry)}
      />
    {/each}
  {/if}

  <button
    type="button"
    onclick={() => workspace.scan()}
    disabled={workspace.scanning}
    class="flex w-full items-center justify-center gap-2 rounded-field
      bg-base-200 py-2 text-sm hover:bg-base-300"
  >
    <Icon icon="lucide:refresh-cw" class="size-4" />
    {m.scan_again()}
  </button>
</div>
