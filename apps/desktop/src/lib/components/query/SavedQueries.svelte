<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon, menu } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  let tab = $state<"saved" | "history">("saved")

  const TABS = [
    { id: "saved" as const, label: m.tab_saved },
    { id: "history" as const, label: m.tab_history },
  ]


  function openMenu(event: MouseEvent, id: string, sql: string) {
    menu.show(event, [
        {
          label: m.tab_query(),
          icon: "lucide:terminal",
          run: () => workspace.query.load(id),
        },
        {
          label: m.menu_run(),
          icon: "lucide:play",
          run: async () => {
            workspace.query.load(id)
            await workspace.query.run()
          },
        },
        {
          label: m.menu_copy(),
          icon: "lucide:copy",
          run: () => navigator.clipboard.writeText(sql),
        },
        {
          label: m.menu_delete(),
          icon: "lucide:x",
          danger: true,
        run: () => workspace.query.drop(id),
      },
    ])
  }
</script>

<aside class="flex w-64 shrink-0 flex-col rounded-box bg-base-100 lift">
  {#if workspace.favorites.length > 0}
    <h2 class="px-4 pt-2 pb-1 text-xs text-base-content/45">
      {m.favorites()}
    </h2>

    <div class="px-2 pb-1">
      {#each workspace.favorites as name (name)}
        <button
          type="button"
          onclick={() => {
            workspace.query.sql = `select * from ${name} limit 100`
          }}
          class="flex w-full items-center gap-2 rounded-field px-2 py-1
            text-left hover:bg-base-200"
          title={name}
        >
          <Icon
            icon="lucide:star"
            class="solid size-4 shrink-0 text-accent"
          />

          <span class="min-w-0 flex-1 truncate text-sm">{name}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="flex gap-1 px-2 pt-2 pb-1">
    {#each TABS as entry (entry.id)}
      <button
        type="button"
        aria-pressed={tab === entry.id}
        onclick={() => (tab = entry.id)}
        class="flex-1 rounded-field px-2 py-1 text-xs {tab === entry.id
          ? 'bg-primary/10 text-primary'
          : 'bg-base-200 hover:bg-base-300'}"
      >
        {entry.label()}
      </button>
    {/each}
  </div>

  <header class="flex items-baseline px-4 pt-1 pb-1">
    <h2 class="flex-1 text-xs text-base-content/45">
      {tab === "saved"
        ? m.saved_count({ count: workspace.query.saved.length })
        : m.history_count({ count: workspace.query.history.length })}
    </h2>

    <button
      type="button"
      onclick={() =>
        tab === "saved"
          ? workspace.query.keep()
          : workspace.query.forgetHistory()}
      class="text-sm text-primary hover:underline"
    >
      {tab === "saved" ? m.keep_this() : m.history_clear()}
    </button>
  </header>

  {#if tab === "history"}
    <div class="flex-1 scroll-smooth overflow-y-auto px-2 pb-2">
      {#each workspace.query.history as entry (entry.id)}
        <button
          type="button"
          ondblclick={() => workspace.query.run()}
          onclick={() => {
            workspace.query.sql = entry.sql
            workspace.query.selection = { start: 0, end: 0 }
          }}
          class="flex w-full items-start gap-2 rounded-field px-2 py-1
            text-left hover:bg-base-200"
        >
          <Icon
            icon={entry.ok ? "lucide:check" : "lucide:x"}
            class="mt-1 size-3 shrink-0 {entry.ok
              ? 'text-success/70'
              : 'text-error'}"
          />

          <span class="min-w-0 flex-1">
            <span class="block truncate text-xs">{entry.sql}</span>
            <span class="block text-xs text-base-content/35">
              {entry.millis} ms{entry.target ? ` · ${entry.target}` : ""}
            </span>
          </span>
        </button>
      {/each}
    </div>
  {:else}
  <div class="flex-1 scroll-smooth overflow-y-auto px-2 pb-2">
    {#each workspace.query.saved as entry (entry.id)}
      <div
        class="group flex items-center rounded-field hover:bg-base-200"
        oncontextmenu={event => openMenu(event, entry.id, entry.sql)}
        role="presentation"
      >
        <button
          type="button"
          onclick={() => workspace.query.load(entry.id)}
          aria-pressed={workspace.query.open === entry.id}
          class="min-w-0 flex-1 px-2 {workspace.density} text-left
            {workspace.query.open === entry.id ? 'text-primary' : ''}"
        >
          <span class="block truncate text-sm">{entry.name}</span>

          {#if entry.target}
            <span class="block truncate text-xs text-base-content/45">
              {entry.target}
            </span>
          {/if}
        </button>

        <button
          type="button"
          aria-label="Drop {entry.name}"
          onclick={() => workspace.query.drop(entry.id)}
          class="mr-2 p-1 text-base-content/30 opacity-0 group-hover:opacity-100
            hover:text-error focus-visible:opacity-100"
        >
          <Icon icon="lucide:x" class="size-4" />
        </button>
      </div>
    {/each}
  </div>
  {/if}
</aside>
