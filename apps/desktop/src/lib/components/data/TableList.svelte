<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { createVirtualizer } from "@tanstack/svelte-virtual"
  import { untrack } from "svelte"

  import { Dropdown, Icon, drag, menu } from "@gpql/ui"
  import { FORMATS, exportTable } from "$lib/session/exporting"
  import { workspace } from "$lib/session/workspace.svelte"

  import ObjectList from "./ObjectList.svelte"
  import type { ExportFormat } from "$lib/types"

  let query = $state("")
  let panel = $state<"tables" | "objects">("tables")

  const PANELS = [
    { id: "tables" as const, label: m.panel_tables },
    { id: "objects" as const, label: m.panel_objects },
  ]
  let scroller = $state<HTMLDivElement | null>(null)

  let shown = $derived.by(() => {
    const matched =
      query.trim() === ""
        ? workspace.tables
        : workspace.tables.filter(table =>
            table.name.toLowerCase().includes(query.trim().toLowerCase()),
          )
    const starred = new Set(workspace.favorites)

    return [
      ...matched.filter(table => starred.has(table.name)),
      ...matched.filter(table => !starred.has(table.name)),
    ]
  })

  const rows = createVirtualizer<HTMLDivElement, HTMLButtonElement>({
    count: 0,
    getScrollElement: () => scroller,
    estimateSize: () => 30,
    overscan: 10,
  })

  $effect(() => {
    const count = shown.length
    const size = workspace.rowHeight
    const element = scroller

    untrack(() => {
      $rows.setOptions({
        count,
        estimateSize: () => size,
        getScrollElement: () => element,
      })
      $rows.measure()
    })
  })


  function openMenu(event: MouseEvent, table: string) {
    menu.show(event, [
      {
        label: workspace.favorites.includes(table)
          ? m.menu_unfavorite()
          : m.menu_favorite(),
        icon: "lucide:star",
        run: () => workspace.toggleFavorite(table),
      },
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
          workspace.query.sql = `select * from ${table} limit 100`
        },
      },
      {
        label: m.menu_ddl(),
        icon: "lucide:file-code-2",
        run: () => workspace.showDdl(table),
      },
      {
        label: m.menu_copy_name(),
        icon: "lucide:copy",
        run: () => navigator.clipboard.writeText(table),
      },
      ...FORMATS.map(format => ({
        label: m.menu_export_as({ format: format.toUpperCase() }),
        icon: "lucide:download",
        run: () => shipOut(table, format),
      })),
])
  }

  // exporting a table takes the filters the user is looking at, not the page
  async function shipOut(table: string, format: ExportFormat) {
    const session = workspace.session

    if (!session) {
      return
    }

    const browse = workspace.browse
    const slice =
      browse.table === table && browse.serverSide
        ? browse.exportSlice()
        : { limit: 0, offset: 0 }

    try {
      workspace.notice = (await exportTable(session.id, table, slice, format)) ?? ""
    } catch (failure) {
      workspace.notice = m.export_failed({ reason: String(failure) })
    }
  }

  function startResize(event: PointerEvent) {
    const startX = event.clientX
    const startWidth = workspace.asideWidth

    drag(
      event,
      moved =>
        (workspace.asideWidth = Math.min(
          Math.max(startWidth + moved.clientX - startX, 192),
          480,
        )),
      () => workspace.setAsideWidth(workspace.asideWidth),
    )
  }

  let loaded = $derived(workspace.browse.result?.rows.length ?? 0)
  let total = $derived(
    workspace.tables.find(table => table.name === workspace.browse.table)?.rows ??
      0,
  )
  let filtered = $derived(
    Object.keys(workspace.browse.filters).length > 0 && workspace.browse.serverSide,
  )

  // the table total says nothing about a filtered set, so do not pair them
  let counter = $derived(
    filtered
      ? m.rows_filtered({ loaded })
      : workspace.browse.end
        ? m.rows_all({ loaded })
        : m.rows_loaded({ loaded, total }),
  )
</script>

<aside
  class="relative flex shrink-0 flex-col rounded-box bg-base-100 lift"
  style:width="{workspace.asideWidth}px"
>
  <button
    type="button"
    aria-label="Resize list"
    onpointerdown={startResize}
    ondblclick={() => (workspace.asideWidth = 256)}
    class="absolute inset-y-0 -right-1 z-10 w-2 cursor-col-resize
      bg-transparent hover:bg-primary/30"
  ></button>

  {#if workspace.schemaNames.length > 0}
    <div class="flex items-center gap-2 px-2 pt-2 pb-1">
      <Dropdown
        wide
        value={workspace.schemaPicked}
        options={workspace.schemaNames.map(name => ({
          value: name,
          label: name,
        }))}
        onpick={name => workspace.useSchema(name)}
      />

      <span class="shrink-0 pr-2 text-xs text-base-content/45">
        {m.tables_count({ count: workspace.tables.length })}
      </span>
    </div>
  {:else}
    <h2 class="px-4 pt-2 pb-1 text-xs text-base-content/45">
      {m.tables_count({ count: workspace.tables.length })}
    </h2>
  {/if}

  {#if workspace.objects.length > 0}
    <div class="flex gap-1 px-2 pb-1">
      {#each PANELS as entry (entry.id)}
        <button
          type="button"
          aria-pressed={panel === entry.id}
          onclick={() => (panel = entry.id)}
          class="flex-1 rounded-field px-2 py-1 text-xs {panel === entry.id
            ? 'bg-primary/10 text-primary'
            : 'bg-base-200 hover:bg-base-300'}"
        >
          {entry.label()}
        </button>
      {/each}
    </div>
  {/if}

  <div class="px-2 pb-1">
    <input
      bind:value={query}
      placeholder={m.search_tables()}
      class="w-full rounded-field bg-base-200 px-2 py-1 text-xs outline-none
        select-text placeholder:text-base-content/30"
    />
  </div>

  {#if panel === "objects"}
    <div class="flex-1 overflow-y-auto pb-2">
      <ObjectList {query} />
    </div>
  {:else}
  <div bind:this={scroller} class="flex-1 scroll-smooth overflow-y-auto px-2">
    <div class="relative" style:height="{$rows.getTotalSize()}px">
      {#each $rows.getVirtualItems() as row (row.key)}
        {@const table = shown[row.index]}

        <button
          type="button"
          onclick={() => workspace.select(table.name)}
          oncontextmenu={event => openMenu(event, table.name)}
          aria-pressed={workspace.browse.table === table.name}
          class="absolute inset-x-0 flex items-center gap-2 rounded-field px-2
            text-left contain-paint transition-colors {workspace.browse.table ===
          table.name
            ? 'bg-primary/10 text-primary'
            : 'hover:bg-base-200'}"
          style:height="{row.size}px"
          style:transform="translateY({row.start}px)"
        >
          <Icon icon="lucide:table-2" class="size-3.5 shrink-0 opacity-60" />

          <span class="flex-1 truncate text-sm" title={table.name}>
            {table.name}
          </span>

          {#if workspace.favorites.includes(table.name)}
            <Icon
              icon="lucide:star"
              class="size-3 shrink-0 fill-current text-accent"
            />
          {/if}

          <span class="text-xs text-base-content/40">{table.rows}</span>
        </button>
      {/each}
    </div>
  </div>
  {/if}

  <p class="px-4 py-2.5 text-xs text-base-content/40">
    {panel === "objects" ? m.objects_count({ count: workspace.objects.length }) : counter}
  </p>
</aside>
