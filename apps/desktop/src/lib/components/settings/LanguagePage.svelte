<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon, RowGroup } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  let busy = $state<string | null>(null)
  let failure = $state("")
  let drafts = $state<Record<string, string>>({})

  const lineOf = (dialect: string) =>
    drafts[dialect] ?? workspace.languageServers[dialect] ?? ""

  let dialects = $derived([
    ...new Set([
      "sql",
      "cypher",
      "flux",
      ...workspace.catalog.map(backend => backend.dialect),
      ...Object.keys(workspace.languageServers),
    ]),
  ])

  async function run(dialect: string, start: boolean) {
    busy = dialect
    failure = ""

    try {
      if (start) {
        await workspace.setLanguageServer(dialect, lineOf(dialect))
        await workspace.startLanguageServer(dialect)
      } else {
        await workspace.stopLanguageServer(dialect)
      }
    } catch (error) {
      failure = String(error)
    } finally {
      busy = null
    }
  }
</script>

<RowGroup>
  {#each dialects as dialect (dialect)}
    {@const line = lineOf(dialect)}
    {@const running = workspace.servers.includes(dialect)}

    <section class="flex flex-col gap-3 px-4 py-4">
      <div class="flex items-center gap-2">
        <Icon icon="lucide:code" class="size-4 shrink-0 text-base-content/70" />

        <h4 class="flex-1 text-sm font-medium">{dialect}</h4>

        <span
          class={[
            "badge badge-sm badge-soft",
            running ? "badge-success" : "badge-neutral",
          ]}
        >
          {running ? m.running() : m.lsp_stopped()}
        </span>
      </div>

      <div class="flex gap-2">
        <input
          value={line}
          aria-label={m.lsp_command({ dialect })}
          placeholder={m.lsp_command_hint()}
          oninput={event => (drafts[dialect] = event.currentTarget.value)}
          onchange={event =>
            workspace.setLanguageServer(dialect, event.currentTarget.value)}
          class="input input-sm min-w-0 flex-1 bg-base-100 select-text"
        />

        {#if running}
          <button
            type="button"
            disabled={busy === dialect}
            onclick={() => run(dialect, false)}
            class="btn btn-ghost btn-sm font-medium hover:text-error"
          >
            {m.lsp_stop()}
          </button>
        {:else}
          <button
            type="button"
            disabled={busy === dialect || line.trim() === ""}
            onclick={() => run(dialect, true)}
            class="btn btn-primary btn-sm font-medium"
          >
            {m.lsp_start()}
          </button>
        {/if}
      </div>
    </section>
  {/each}
</RowGroup>

{#if workspace.lspError || failure}
  <p class="text-xs wrap-anywhere text-error select-text">
    {workspace.lspError || failure}
  </p>
{/if}
