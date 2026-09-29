<script lang="ts">
  import { onMount } from "svelte"

  import { arrive, Icon, ListRow, Marker } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { blankConfig } from "$lib/session/commands"
  import { hint } from "$lib/session/errors"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Discovery, SessionConfig } from "$lib/types"

  import { withoutLogin } from "./address"
  import Pending from "./Pending.svelte"

  type Props = { onhandoff: (config: SessionConfig) => void }

  let { onhandoff }: Props = $props()

  let opening = $state<string | null>(null)

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

  const keyOf = (entry: Discovery) =>
    [entry.kind, entry.host, entry.port, entry.database].join("\n")

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

  async function pick(entry: Discovery) {
    if (entry.needsLogin) {
      onhandoff(toConfig(entry))

      return
    }

    if (opening) {
      return
    }

    opening = keyOf(entry)

    await workspace.open(toConfig(entry)).catch(() => undefined)

    opening = null
  }

  let advice = $derived(hint(workspace.tailnetError))
</script>

{#snippet results(found: Discovery[])}
  <div class="flex flex-col pb-2">
    {#each found as entry, index (keyOf(entry))}
      <div
        in:arrive|global={{ from: "down", distance: 0.5, delay: index * 30 }}
      >
        <ListRow
          icon={entry.needsLogin
            ? "lucide:lock"
            : workspace.iconFor(entry.kind)}
          title={entry.database ||
            m.scan_on_port({ kind: labelOf(entry), port: entry.port })}
          detail={withoutLogin(entry.detail) ||
            (entry.user
              ? `${entry.user}@${entry.host}:${entry.port}`
              : `${entry.host}:${entry.port}`)}
          trailing={entry.needsLogin
            ? "lucide:pencil"
            : "lucide:arrow-right"}
          busy={opening === keyOf(entry)}
          onclick={() => pick(entry)}
        />
      </div>
    {/each}
  </div>
{/snippet}

{#snippet heading(
  label: string,
  icon: string,
  action: string,
  busy: boolean,
  run: () => void,
)}
  <div class="flex shrink-0 items-center gap-2 px-4 pt-4 pb-2">
    <Marker as="h2" {label} class="flex-1" />

    <button
      type="button"
      onclick={run}
      disabled={busy}
      class="btn btn-ghost btn-xs font-medium"
    >
      <Icon {icon} class={["size-4", busy && "animate-spin"]} />
      {action}
    </button>
  </div>
{/snippet}

{#snippet blank(text: string)}
  <p
    class={[
      "grid h-full place-items-center px-4 pb-4 text-center text-sm",
      "text-base-content/70",
    ]}
  >
    {text}
  </p>
{/snippet}

<section class="flex min-h-0 flex-1 flex-col">
  {@render heading(
    m.scan_local_title(),
    "lucide:refresh-cw",
    m.scan_again(),
    workspace.scanning,
    () => workspace.scan(),
  )}

  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if workspace.found.length > 0}
      {@render results(workspace.found)}
    {:else if workspace.scanning}
      <Pending label={m.scan_running()} />
    {:else}
      {@render blank(m.scan_empty())}
    {/if}
  </div>
</section>

<section class="flex min-h-0 flex-1 flex-col border-t border-base-content/10">
  {@render heading(
    m.tailnet_title(),
    "lucide:radar",
    m.tailnet_scan(),
    workspace.scanningTailnet,
    () => workspace.scanTailnet(),
  )}

  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if workspace.tailnetError}
      <div class="flex flex-col gap-1 px-4 py-2">
        <p class="text-xs wrap-anywhere text-error select-text">
          {workspace.tailnetError}
        </p>

        {#if advice}
          <p class="text-xs text-base-content/70">{advice}</p>
        {/if}
      </div>
    {:else if workspace.tailnet.length > 0}
      {@render results(workspace.tailnet)}
    {:else if workspace.scanningTailnet}
      <Pending label={m.tailnet_running()} rows={2} />
    {:else}
      {@render blank(m.tailnet_empty())}
    {/if}
  </div>
</section>
