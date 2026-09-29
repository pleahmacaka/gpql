<script lang="ts">
  import {
    Dropdown,
    EmptyState,
    Icon,
    Marker,
    pop,
    tooltip,
    veil,
  } from "@gpql/ui"
  import { untrack } from "svelte"
  import { fade, scale } from "svelte/transition"

  import * as m from "$lib/paraglide/messages"
  import type { Connection } from "$lib/session/connection.svelte"
  import type { TableDiff } from "$lib/session/diff"
  import { workspace } from "$lib/session/workspace.svelte"

  let against = $state("")
  let copied = $state(false)

  let here = $derived(workspace.active)

  let others = $derived(
    workspace.connections.filter(entry => entry.id !== here?.id),
  )

  let other = $derived(others.find(entry => entry.id === against) ?? others[0])

  let sides = $derived(
    [here, other].filter((entry): entry is Connection => entry != null),
  )

  $effect(() => {
    for (const side of sides) {
      if (side.schemaState === "idle") {
        untrack(() => void side.loadSchema())
      }
    }
  })

  let failure = $derived(
    sides.find(side => side.schemaState === "failed")?.schemaError ?? null,
  )

  let report = $derived(other ? workspace.diffAgainst(other) : null)

  let bare = $derived(
    report ? sides.filter(side => side.schema.length === 0) : [],
  )

  let sql = $derived(report?.sql.join("\n\n") ?? "")

  let tally = $derived.by(() => {
    const count = (state: TableDiff["state"]) =>
      report?.tables.filter(entry => entry.state === state).length ?? 0

    return {
      added: count("added"),
      changed: count("changed"),
      dropped: count("dropped"),
    }
  })

  const MARK = {
    added: { icon: "lucide:plus", tone: "bg-success text-success-content" },
    dropped: { icon: "lucide:minus", tone: "bg-error text-error-content" },
    changed: { icon: "lucide:pencil", tone: "bg-warning text-warning-content" },
  }

  function retry() {
    for (const side of sides) {
      if (side.schemaState === "failed") {
        void side.loadSchema(true)
      }
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(sql)
    copied = true
    setTimeout(() => (copied = false), 1400)
  }

  function send() {
    if (other && sql !== "") {
      void workspace.draftMigration(other.id, sql)
    }
  }
</script>

{#snippet mark(table: TableDiff["state"], state: TableDiff["state"])}
  {#if table === "changed"}
    {@render sign(state)}
  {:else}
    <span aria-hidden="true" class="size-4 shrink-0"></span>
  {/if}
{/snippet}

{#snippet sign(state: TableDiff["state"])}
  <span
    aria-hidden="true"
    class={["grid size-4 shrink-0 place-items-center", MARK[state].tone]}
  >
    <Icon icon={MARK[state].icon} class="size-3" />
  </span>
{/snippet}

<div class="@container flex min-h-0 flex-1 flex-col">
  <header
    class={[
      "flex h-12 shrink-0 items-center gap-2 border-b border-base-content/10",
      "px-4",
    ]}
  >
    <Icon icon="lucide:git-compare" class="size-4 shrink-0 text-primary" />

    <span class="shrink-0 text-xs text-base-content/70">
      {m.diff_against()}
    </span>

    {#if others.length > 0}
      <Dropdown
        small
        label={m.diff_against()}
        value={other?.id ?? ""}
        options={others.map(entry => ({
          value: entry.id,
          label: entry.label,
        }))}
        onpick={id => (against = id)}
      />
    {/if}

    {#if here && other}
      <p
        class="hidden min-w-0 truncate text-xs text-base-content/70 @3xl:block"
      >
        {m.diff_direction({ other: other.label, here: here.label })}
      </p>
    {/if}

    <span class="flex-1"></span>

    {#if report && report.tables.length > 0}
      <p
        in:fade={veil()}
        class={[
          "flex shrink-0 items-center gap-3 text-xs whitespace-nowrap",
          "tabular-nums",
        ]}
      >
        <span class="flex items-center gap-1">
          {@render sign("added")}
          {m.diff_added({ count: tally.added })}
        </span>

        <span class="flex items-center gap-1">
          {@render sign("changed")}
          {m.diff_changed({ count: tally.changed })}
        </span>

        <span class="flex items-center gap-1">
          {@render sign("dropped")}
          {m.diff_dropped({ count: tally.dropped })}
        </span>
      </p>
    {/if}
  </header>

  <div class="relative min-h-0 flex-1">
    {#if others.length === 0}
      <div in:fade={veil()} class="absolute inset-0 grid place-items-center">
        <EmptyState art="link" title={m.diff_needs_two()} />
      </div>
    {:else if failure}
      <div
        in:fade={veil()}
        class="absolute inset-0 grid place-items-center overflow-y-auto"
      >
        <EmptyState
          art="link"
          title={m.schema_failed()}
          hint={failure}
          class="select-text"
        >
          <button
            type="button"
            onclick={retry}
            class="btn btn-soft btn-sm font-medium"
          >
            <Icon icon="lucide:rotate-cw" class="size-4" />
            {m.diff_retry()}
          </button>
        </EmptyState>
      </div>
    {:else if !report}
      <div
        role="status"
        in:fade={veil()}
        class="absolute inset-0 grid place-items-center"
      >
        <EmptyState art="sheet" title={m.diff_loading()} />
      </div>
    {:else if report.tables.length === 0}
      <div in:fade={veil()} class="absolute inset-0 grid place-items-center">
        <EmptyState
          art={null}
          title={bare.length > 0
            ? m.diff_empty({ name: bare[0].label })
            : m.diff_same()}
        />
      </div>
    {:else}
      <div
        in:fade={veil()}
        class={[
          "absolute inset-0 grid grid-rows-2 @3xl:grid-cols-2",
          "@3xl:grid-rows-1",
        ]}
      >
        <section
          aria-label={m.diff_changes()}
          class={[
            "flex min-h-0 flex-col border-b border-base-content/10",
            "@3xl:border-r @3xl:border-b-0",
          ]}
        >
          <header class="flex h-10 shrink-0 items-center px-4">
            <Marker as="h3" label={m.diff_changes()} />
          </header>

          {#each bare as side (side.id)}
            <p class="px-4 pb-2 text-xs text-base-content/70">
              {m.diff_empty({ name: side.label })}
            </p>
          {/each}

          <ul
            class={[
              "flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-4 pb-4",
              "select-text",
            ]}
          >
            {#each report.tables as entry, index (index)}
              <li class="shrink-0 bg-base-100 hairline">
                <p
                  class={[
                    "flex h-8 items-center gap-2 px-3 text-sm font-medium",
                    "border-b border-base-content/10",
                  ]}
                >
                  {@render sign(entry.state)}
                  <span class="truncate">{entry.table}</span>
                </p>

                <ul class="py-1 text-xs">
                  {#each entry.addedColumns as column, index (index)}
                    <li class="flex h-6 items-center gap-2 px-3">
                      {@render mark(entry.state, "added")}
                      <span class="min-w-0 flex-1 truncate">{column.name}</span>
                      <span class="shrink-0 text-base-content/70">
                        {column.dataType}
                      </span>
                    </li>
                  {/each}

                  {#each entry.droppedColumns as column, index (index)}
                    <li class="flex h-6 items-center gap-2 px-3">
                      {@render mark(entry.state, "dropped")}
                      <span class="min-w-0 flex-1 truncate">{column.name}</span>
                      <span class="shrink-0 text-base-content/70">
                        {column.dataType}
                      </span>
                    </li>
                  {/each}

                  {#each entry.changedColumns as change, index (index)}
                    <li class="flex min-h-6 items-center gap-2 px-3 py-1">
                      {@render sign("changed")}
                      <span class="shrink-0">{change.name}</span>
                      <span class="min-w-0 flex-1 text-base-content/70">
                        {change.was}
                        <Icon
                          icon="lucide:arrow-right"
                          class="inline size-3 align-middle"
                        />
                        {change.now}
                      </span>
                    </li>
                  {/each}
                </ul>
              </li>
            {/each}
          </ul>
        </section>

        <section aria-label={m.diff_draft()} class="flex min-h-0 flex-col">
          <header class="flex h-10 shrink-0 items-center gap-1 pr-2 pl-4">
            <Marker as="h3" label={m.diff_draft()} class="min-w-0 flex-1" />

            <button
              type="button"
              aria-label={m.menu_copy()}
              use:tooltip={m.menu_copy()}
              disabled={sql === ""}
              onclick={copy}
              class="btn btn-square btn-ghost btn-sm"
            >
              {#key copied}
                <span in:scale={pop()} class="grid place-items-center">
                  <Icon
                    icon={copied ? "lucide:check" : "lucide:copy"}
                    class={["size-4", copied && "text-success"]}
                  />
                </span>
              {/key}
            </button>

            {#if other}
              <button
                type="button"
                disabled={sql === ""}
                onclick={send}
                class="btn btn-primary btn-sm max-w-56 font-medium"
              >
                <Icon icon="lucide:file-code-2" class="size-4" />
                <span class="truncate">
                  {m.diff_open({ name: other.label })}
                </span>
              </button>
            {/if}
          </header>

          <p class="px-4 pb-2 text-xs text-pretty text-base-content/70">
            {m.diff_draft_hint()}
          </p>

          <div
            class={[
              "mx-4 mb-4 flex min-h-0 flex-1 flex-col gap-3 overflow-auto",
              "bg-base-200 p-3 text-xs leading-5 select-text hairline",
            ]}
          >
            {#each report.sql as statement, index (index)}
              <p
                class={[
                  "whitespace-pre-wrap",
                  statement.startsWith("--") && "text-base-content/70 italic",
                ]}
              >
                {statement}
              </p>
            {/each}
          </div>
        </section>
      </div>
    {/if}
  </div>
</div>
