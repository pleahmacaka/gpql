<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon } from "@gpql/ui"

  import { workspace } from "$lib/session/workspace.svelte"

  const dialects = [
    { id: "sql", label: "SQL", hint: "sqls, postgres-lsp, sql-language-server" },
    { id: "cypher", label: "Cypher", hint: "cypher-language-server --stdio" },
  ]

  let note = $state("")

  async function apply(dialect: string, line: string) {
    note = "starting"

    try {
      await workspace.startLanguageServer(dialect, line)
      note = workspace.servers.includes(dialect)
        ? `${dialect} server is answering`
        : `${dialect} server stopped`
    } catch (failure) {
      note = String(failure)
    }
  }
</script>

<p class="text-xs text-base-content/45">
  {m.editor_note()}
</p>

<div class="mt-2 space-y-2">
  {#each dialects as dialect (dialect.id)}
    <div class="rounded-field bg-base-200 p-2">
      <div class="flex items-center gap-2 pb-1">
        <Icon
          icon="lucide:circle-dot"
          class="size-3 {workspace.servers.includes(dialect.id)
            ? 'text-success'
            : 'text-base-content/25'}"
        />

        <span class="flex-1 text-sm">{dialect.label}</span>
      </div>

      <div class="flex gap-1">
        <input
          value={workspace.languageServers[dialect.id] ?? ""}
          oninput={event =>
            (workspace.languageServers[dialect.id] = event.currentTarget.value)}
          placeholder={dialect.hint}
          class="min-w-0 flex-1 rounded-field bg-base-100 px-2 py-1 text-xs
            outline-none placeholder:text-base-content/30"
        />

        <button
          type="button"
          onclick={() =>
            apply(dialect.id, workspace.languageServers[dialect.id] ?? "")}
          class="rounded-field bg-base-100 px-2 py-1 text-xs hover:bg-base-300"
        >
          {m.apply()}
        </button>
      </div>
    </div>
  {/each}
</div>

{#if note}
  <p class="px-1 pt-2 text-xs text-base-content/45">{note}</p>
{/if}
