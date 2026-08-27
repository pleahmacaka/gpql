<script lang="ts">
  import { Dropdown, Icon } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { diffSchemas } from "$lib/session/diff"
  import { workspace } from "$lib/session/workspace.svelte"

  let against = $state("")
  let busy = $state(false)

  let others = $derived(
    workspace.connections.filter(entry => entry.id !== workspace.active?.id),
  )

  let other = $derived(others.find(entry => entry.id === against) ?? others[0])

  // both sides have to be loaded before anything can be compared
  $effect(() => {
    const target = other

    if (!target || target.schema.length > 0 || busy) {
      return
    }

    busy = true
    target
      .loadSchema()
      .finally(() => (busy = false))
  })

  let report = $derived(
    other && other.schema.length > 0
      ? diffSchemas(other.schema, workspace.schema)
      : null,
  )

  const TONE = {
    added: "text-success",
    dropped: "text-error",
    changed: "text-warning",
  }
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2 p-2">
  <header class="flex items-center gap-2 px-2">
    <Icon icon="lucide:git-compare" class="size-4 text-base-content/40" />

    <span class="text-xs text-base-content/45">{m.diff_against()}</span>

    {#if others.length > 0}
      <Dropdown
        value={other?.id ?? ""}
        options={others.map(entry => ({
          value: entry.id,
          label: entry.label,
        }))}
        onpick={id => (against = id)}
      />
    {/if}

    <span class="flex-1"></span>

    {#if report && report.sql.length > 0}
      <button
        type="button"
        onclick={() => {
          workspace.query.sql = report?.sql.join("\n\n") ?? ""
          workspace.tab = "query"
        }}
        class="rounded-field bg-primary px-2 py-1 text-xs text-primary-content"
      >
        {m.diff_to_query()}
      </button>
    {/if}
  </header>

  {#if others.length === 0}
    <p class="px-3 py-6 text-center text-sm text-base-content/45">
      {m.diff_needs_two()}
    </p>
  {:else if busy || !report}
    <p class="px-3 py-6 text-center text-sm text-base-content/45">
      <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
    </p>
  {:else if report.tables.length === 0}
    <p class="px-3 py-6 text-center text-sm text-base-content/45">
      {m.diff_same()}
    </p>
  {:else}
    <div class="min-h-0 flex-1 overflow-y-auto px-2 select-text">
      {#each report.tables as entry (entry.table)}
        <div class="rounded-field bg-base-200 px-3 py-2">
          <p class="flex items-center gap-2 text-sm {TONE[entry.state]}">
            <Icon
              icon={entry.state === "added"
                ? "lucide:plus"
                : entry.state === "dropped"
                  ? "lucide:minus"
                  : "lucide:pencil"}
              class="size-3.5 shrink-0"
            />
            {entry.table}
          </p>

          {#each entry.addedColumns as column (column.name)}
            <p class="pl-6 text-xs text-success">
              + {column.name} {column.dataType}
            </p>
          {/each}

          {#each entry.droppedColumns as column (column.name)}
            <p class="pl-6 text-xs text-error">
              − {column.name} {column.dataType}
            </p>
          {/each}

          {#each entry.changedColumns as change (change.name)}
            <p class="pl-6 text-xs text-warning">
              ~ {change.name}: {change.was} → {change.now}
            </p>
          {/each}
        </div>
      {/each}

      <pre
        class="mt-2 rounded-field bg-base-200 p-3 text-xs whitespace-pre-wrap">{report.sql.join(
          "\n",
        )}</pre>
    </div>
  {/if}
</div>
