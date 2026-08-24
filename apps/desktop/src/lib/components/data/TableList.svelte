<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { ContextMenu, Icon } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  let menu = $state<{
    x: number
    y: number
    items: { label: string; icon?: string; danger?: boolean; run: () => void }[]
  } | null>(null)

  function openMenu(event: MouseEvent, table: string) {
    event.preventDefault()

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        {
          label: m.tab_data(),
          icon: "lucide:table-2",
          run: async () => {
            workspace.tab = "data"
            await workspace.select(table)
          },
        },
        {
          label: m.tab_schema(),
          icon: "lucide:git-fork",
          run: async () => {
            workspace.tab = "schema"
            await workspace.loadSchema()
            await workspace.select(table)
          },
        },
        {
          label: m.menu_select_from(),
          icon: "lucide:terminal",
          run: () => {
            workspace.tab = "query"
            workspace.sql = `select * from ${table} limit 100`
          },
        },
        {
          label: m.menu_copy_name(),
          icon: "lucide:copy",
          run: () => navigator.clipboard.writeText(table),
        },
      ],
    }
  }

  let loaded = $derived(workspace.rows?.rows.length ?? 0)
  let total = $derived(
    workspace.tables.find(table => table.name === workspace.selected)?.rows ?? 0,
  )
</script>

<aside class="flex w-64 shrink-0 flex-col rounded-box bg-base-100 lift">
  <h2 class="px-4 pt-2 pb-1 text-xs text-base-content/45">
    {m.tables_count({ count: workspace.tables.length })}
  </h2>

  <div class="flex-1 overflow-y-auto px-2">
    {#each workspace.tables as table (table.name)}
      <button
        type="button"
        onclick={() => workspace.select(table.name)}
        oncontextmenu={event => openMenu(event, table.name)}
        aria-pressed={workspace.selected === table.name}
        class="flex w-full items-center gap-2 rounded-field px-2 {workspace.density}
          text-left transition-colors {workspace.selected === table.name
          ? 'bg-primary/10 text-primary'
          : 'hover:bg-base-200'}"
      >
        <Icon icon="lucide:table-2" class="size-3.5 shrink-0 opacity-60" />

        <span class="flex-1 truncate text-sm">{table.name}</span>

        <span class="text-xs text-base-content/40">{table.rows}</span>
      </button>
    {/each}
  </div>

  <p class="px-4 py-2.5 text-xs text-base-content/40">
    {loaded} of {total}
  </p>

  {#if menu}
    <ContextMenu
      x={menu.x}
      y={menu.y}
      items={menu.items}
      onclose={() => (menu = null)}
    />
  {/if}
</aside>
