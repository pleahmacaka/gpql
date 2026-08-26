<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { ContextMenu, Icon } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  let menu = $state<{
    x: number
    y: number
    items: { label: string; icon?: string; danger?: boolean; run: () => void }[]
  } | null>(null)

  function openMenu(event: MouseEvent, id: string, sql: string) {
    event.preventDefault()

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        {
          label: m.tab_query(),
          icon: "lucide:terminal",
          run: () => {
            workspace.sql = sql
          },
        },
        {
          label: m.menu_run(),
          icon: "lucide:play",
          run: async () => {
            workspace.sql = sql
            await workspace.run()
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
          run: () => workspace.drop(id),
        },
      ],
    }
  }
</script>

<aside class="flex w-64 shrink-0 flex-col rounded-box bg-base-100 lift">
  <header class="flex items-baseline px-4 pt-2 pb-1">
    <h2 class="flex-1 text-xs text-base-content/45">
      {m.saved_count({ count: workspace.saved.length })}
    </h2>

    <button
      type="button"
      onclick={() => workspace.keep()}
      class="text-sm text-primary hover:underline"
    >
      {m.keep_this()}
    </button>
  </header>

  <div class="flex-1 scroll-smooth overflow-y-auto px-2 pb-2">
    {#each workspace.saved as entry (entry.id)}
      <div
        class="group flex items-center rounded-field hover:bg-base-200"
        oncontextmenu={event => openMenu(event, entry.id, entry.sql)}
        role="presentation"
      >
        <button
          type="button"
          onclick={() => (workspace.sql = entry.sql)}
          class="min-w-0 flex-1 px-2 {workspace.density} text-left"
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
          onclick={() => workspace.drop(entry.id)}
          class="mr-2 p-1 text-base-content/30 opacity-0 group-hover:opacity-100
            hover:text-error focus-visible:opacity-100"
        >
          <Icon icon="lucide:x" class="size-3.5" />
        </button>
      </div>
    {/each}
  </div>

  {#if menu}
    <ContextMenu
      x={menu.x}
      y={menu.y}
      items={menu.items}
      onclose={() => (menu = null)}
    />
  {/if}
</aside>
