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

  function labelOf(entry: Discovery) {
    return (
      workspace.catalog.find(backend => backend.id === entry.kind)?.label ??
      entry.kind
    )
  }

  function toConfig(entry: Discovery): SessionConfig {
    const config: SessionConfig = {
      ...blankConfig(),
      kind: entry.kind,
      host: entry.host,
      port: entry.port,
      user: entry.user,
      password: entry.password,
      database: entry.database,
    }

    switch (entry.kind) {
      case "mqtt":
        return { ...config, database: "#" }
      case "falkordb":
        return { ...config, url: `redis://${entry.host}:${entry.port}` }
      case "s3":
      case "clickhouse":
      case "influxdb":
      case "influxdb2":
      case "influxdb3":
        return { ...config, url: `http://${entry.host}:${entry.port}` }
      case "neo4j":
        return { ...config, url: `neo4j://${entry.host}:${entry.port}` }
      default:
        return config
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
    {#each workspace.found as entry (entry.kind + entry.host + entry.port + entry.database)}
      <ListRow
        icon={entry.needsLogin ? "lucide:lock" : workspace.iconFor(entry.kind)}
        title={entry.database || `${labelOf(entry)} on ${entry.port}`}
        detail={entry.detail ||
          (entry.user
            ? `${entry.user}@${entry.host}:${entry.port}`
            : `${entry.host}:${entry.port}`)}
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
