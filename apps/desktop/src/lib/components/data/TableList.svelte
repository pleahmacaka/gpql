<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { createVirtualizer } from "@tanstack/svelte-virtual"
  import { open } from "@tauri-apps/plugin-dialog"
  import { tick, untrack } from "svelte"
  import { fade } from "svelte/transition"

  import {
    arrive,
    contextmenu,
    depart,
    Dropdown,
    drag,
    Icon,
    type MenuItem,
    Panel,
    rem,
    Segmented,
    tooltip,
    veil,
  } from "@gpql/ui"
  import { selectFrom } from "$lib/components/query/buffer"
  import { exportTable, FORMATS } from "$lib/session/exporting"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { ExportFormat, TableInfo } from "$lib/types"

  import ObjectList from "./ObjectList.svelte"
  import TopicTree from "./TopicTree.svelte"

  type Props = { onblocked?: () => void }

  let { onblocked }: Props = $props()

  const ASIDE = 16

  const unit = rem(1)

  let query = $state("")
  let chosen = $state<"tables" | "objects">("tables")
  let switching = $state(false)

  let panel = $derived(workspace.objects.length > 0 ? chosen : "tables")

  $effect(() => {
    void workspace.activeId
    chosen = "tables"
  })

  let mqtt = $derived(workspace.session?.kind === "mqtt")
  let s3 = $derived(workspace.session?.kind === "s3")

  let panels = $derived([
    { value: "tables", label: workspace.nouns.panel() },
    { value: "objects", label: m.panel_objects() },
  ])

  let scroller = $state<HTMLDivElement | null>(null)

  let starred = $derived(new Set(workspace.favorites))

  let shown = $derived.by(() => {
    const needle = query.trim().toLowerCase()
    const matched =
      needle === ""
        ? workspace.tables
        : workspace.tables.filter(table =>
            table.name.toLowerCase().includes(needle),
          )

    return [
      ...matched.filter(table => starred.has(table.name)),
      ...matched.filter(table => !starred.has(table.name)),
    ]
  })

  let lastStar = $derived(
    shown.findLastIndex(table => starred.has(table.name)),
  )

  const compact = new Intl.NumberFormat(workspace.locale, {
    notation: "compact",
    maximumFractionDigits: 1,
  })

  const rows = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => scroller,
    estimateSize: () => rem(2),
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

  let skeleton = $derived(
    Math.max(Math.ceil(rem(20) / workspace.rowHeight), 1),
  )

  function itemsFor(table: string): MenuItem[] {
    const favorite = {
      label: workspace.favorites.includes(table)
        ? m.menu_unfavorite()
        : m.menu_favorite(),
      icon: "lucide:star",
      run: () => workspace.toggleFavorite(table),
    }

    const copyName = {
      label: m.menu_copy_name(),
      icon: "lucide:copy",
      run: () => navigator.clipboard.writeText(table),
    }

    const exports = FORMATS.map(format => ({
      label: m.menu_export_as({ format: format.toUpperCase() }),
      icon: "lucide:download",
      run: () => shipOut(table, format),
    }))

    if (s3) {
      return [
        favorite,
        {
          label: m.s3_refresh(),
          icon: "lucide:refresh-cw",
          run: () => workspace.active?.s3Refresh(table),
        },
        {
          label: m.s3_upload(),
          icon: "lucide:upload",
          run: () => uploadInto(table),
        },
        copyName,
        ...exports,
      ]
    }

    return [
      favorite,
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
          void workspace.query.replace(
            selectFrom(table, workspace.session?.kind ?? "", workspace.dialect),
          )
        },
      },
      {
        label: m.menu_ddl(),
        icon: "lucide:file-code-2",
        run: () => workspace.showDdl(table),
      },
      copyName,
      ...exports,
    ]
  }

  async function uploadInto(bucket: string) {
    if (workspace.readOnly) {
      onblocked?.()

      return
    }

    const path = await open({ multiple: false })
    const connection = workspace.active

    if (typeof path !== "string" || !connection) {
      return
    }

    const key = path.split(/[\\/]/).pop() ?? path

    try {
      await connection.s3Upload(bucket, key, path)
      workspace.notice = m.s3_sent({ key })
    } catch (failure) {
      workspace.notice = String(failure)
    }
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
      workspace.notice =
        (await exportTable(session.id, table, slice, format)) ?? ""
    } catch (failure) {
      workspace.notice = m.export_failed({ reason: String(failure) })
    }
  }

  async function useSchema(name: string) {
    switching = true

    try {
      await workspace.useSchema(name)
    } finally {
      switching = false
    }
  }

  function startResize(event: PointerEvent) {
    const startX = event.clientX
    const startWidth = workspace.asideWidth

    drag(
      event,
      moved =>
        (workspace.asideWidth = Math.min(
          Math.max(startWidth + moved.clientX - startX, rem(12)),
          rem(30),
        )),
      () => workspace.setAsideWidth(workspace.asideWidth),
    )
  }

  async function reach(index: number) {
    const bounded = Math.max(0, Math.min(index, shown.length - 1))

    $rows.scrollToIndex(bounded, { align: "auto" })
    await tick()
    requestAnimationFrame(() =>
      scroller
        ?.querySelector<HTMLElement>(`[data-index="${bounded}"] button`)
        ?.focus(),
    )
  }

  function listKeys(event: KeyboardEvent) {
    const row = (event.target as HTMLElement | null)?.closest<HTMLElement>(
      "[data-index]",
    )

    if (!row) {
      return
    }

    const index = Number(row.dataset.index)
    const moves: Record<string, number> = {
      ArrowDown: index + 1,
      ArrowUp: index - 1,
      Home: 0,
      End: shown.length - 1,
    }

    if (event.key in moves) {
      event.preventDefault()
      void reach(moves[event.key])
    }
  }

  function searchKeys(event: KeyboardEvent) {
    if (event.isComposing) {
      return
    }

    if (event.key === "Escape" && query !== "") {
      event.preventDefault()
      event.stopPropagation()
      query = ""

      return
    }

    if (event.key === "ArrowDown" && panel === "tables" && !mqtt) {
      event.preventDefault()
      void reach(0)

      return
    }

    if (event.key === "Enter" && panel === "tables" && !mqtt && shown[0]) {
      event.preventDefault()
      void workspace.select(shown[0].name)
    }
  }

  function countOf(table: TableInfo) {
    return table.rows > 0 ? compact.format(table.rows) : ""
  }
</script>

<div
  class="relative flex shrink-0"
  style:width="{workspace.asideWidth / unit}rem"
>
  <Panel glass={false} label={workspace.nouns.panel()} class="min-w-0 flex-1">
    {#if workspace.schemaNames.length > 0}
      <div class="px-3 pt-3">
        <Dropdown
          wide
          label={m.schema_label()}
          value={workspace.schemaPicked}
          options={workspace.schemaNames.map(name => ({
            value: name,
            label: name,
          }))}
          onpick={useSchema}
        />
      </div>
    {/if}

    {#if workspace.objects.length > 0}
      <div class="px-3 pt-3">
        <Segmented
          small
          label={workspace.nouns.panel()}
          options={panels}
          value={panel}
          onpick={next => (chosen = next === "objects" ? "objects" : "tables")}
        />
      </div>
    {/if}

    <div class="p-3">
      <label class="input input-sm w-full bg-base-100">
        <Icon
          icon="lucide:search"
          class="size-4 shrink-0 text-base-content/60"
        />

        <input
          bind:value={query}
          onkeydown={searchKeys}
          placeholder={workspace.nouns.search()}
          aria-label={workspace.nouns.search()}
          spellcheck="false"
          class="min-w-0 grow select-text placeholder:text-base-content/60"
        />

        {#if query !== ""}
          <button
            type="button"
            transition:fade|local={veil()}
            aria-label={m.search_clear()}
            onclick={() => (query = "")}
            class={[
              "grid size-4 shrink-0 cursor-pointer place-items-center",
              "text-base-content/60 hover:text-base-content",
            ]}
          >
            <Icon icon="lucide:x" class="size-4" />
          </button>
        {/if}
      </label>
    </div>

    <div
      class={[
        "relative min-h-0 flex-1 overflow-hidden border-t",
        "border-base-content/10",
      ]}
    >
      {#if panel === "objects"}
        <div
          in:arrive={{ from: "right" }}
          out:depart={{ to: "right" }}
          class="absolute inset-0 overflow-y-auto py-1"
          style:scrollbar-gutter="stable"
        >
          <ObjectList {query} />
        </div>
      {:else if mqtt}
        <div class="absolute inset-0 flex flex-col">
          <TopicTree {query} onexport={shipOut} />
        </div>
      {:else if switching}
        <div
          aria-hidden="true"
          class="absolute inset-0 flex flex-col overflow-hidden py-1"
        >
          {#each { length: skeleton }, line (line)}
            <div
              class="flex shrink-0 items-center gap-3 px-3"
              style:height="{workspace.rowHeight / unit}rem"
            >
              <span class="skeleton size-4 shrink-0 opacity-60"></span>
              <span
                class={[
                  "skeleton h-2 opacity-60",
                  ["w-24", "w-32", "w-20", "w-28"][line % 4],
                ]}
              ></span>
            </div>
          {/each}
        </div>
      {:else}
        <div
          bind:this={scroller}
          in:arrive={{ from: "left" }}
          out:depart={{ to: "left" }}
          onkeydown={listKeys}
          role="presentation"
          class="absolute inset-0 overflow-y-auto py-1"
          style:scrollbar-gutter="stable"
        >
          {#if shown.length === 0}
            <p class="px-4 py-6 text-center text-xs text-base-content/70">
              {query.trim() === "" ? workspace.nouns.none() : m.no_match()}
            </p>
          {/if}

          <div
            class="relative"
            style:height="{$rows.getTotalSize() / unit}rem"
          >
            {#each $rows.getVirtualItems() as row (row.key)}
              {@const table = shown[row.index]}

              {#if table}
                {@const picked = workspace.browse.table === table.name}
                {@const star = starred.has(table.name)}

                <div
                  data-index={row.index}
                  use:contextmenu={() => itemsFor(table.name)}
                  class={[
                    "group absolute inset-x-0 flex items-center pr-2",
                    "contain-paint transition-colors",
                    picked
                      ? "bg-primary/10 text-primary"
                      : "hover:bg-base-content/5",
                    row.index === lastStar &&
                      lastStar < shown.length - 1 &&
                      "border-b border-base-content/10",
                  ]}
                  style:height="{row.size / unit}rem"
                  style:transform="translateY({row.start / unit}rem)"
                >
                  {#if picked}
                    <span
                      aria-hidden="true"
                      class="absolute inset-y-0 left-0 w-1 bg-primary"
                    ></span>
                  {/if}

                  <button
                    type="button"
                    onclick={() => workspace.select(table.name)}
                    aria-current={picked ? "true" : undefined}
                    class={[
                      "flex h-full min-w-0 flex-1 cursor-pointer items-center",
                      "gap-2 pl-4 text-left outline-offset-0",
                    ]}
                  >
                    <Icon
                      icon={s3 ? "lucide:package" : "lucide:table-2"}
                      class={[
                        "size-4 shrink-0",
                        picked ? "text-primary" : "text-base-content/60",
                      ]}
                    />

                    <span
                      class={["truncate text-sm", picked && "font-medium"]}
                      title={table.name}
                    >
                      {table.name}
                    </span>
                  </button>

                  {#if countOf(table)}
                    <span
                      use:tooltip={workspace.nouns.row(table.rows)}
                      class={[
                        "shrink-0 pl-2 text-xs tabular-nums",
                        picked ? "text-primary" : "text-base-content/70",
                      ]}
                    >
                      {countOf(table)}
                    </span>
                  {/if}

                  <button
                    type="button"
                    tabindex="-1"
                    aria-label={star ? m.menu_unfavorite() : m.menu_favorite()}
                    aria-pressed={star}
                    onclick={() => workspace.toggleFavorite(table.name)}
                    class={[
                      "grid size-6 shrink-0 cursor-pointer place-items-center",
                      "transition-opacity",
                      star
                        ? "text-accent"
                        : [
                            "text-base-content/60 opacity-0",
                            "group-hover:opacity-100 hover:text-accent",
                          ],
                    ]}
                  >
                    <Icon
                      icon="lucide:star"
                      class={["size-4", star && "solid"]}
                    />
                  </button>
                </div>
              {/if}
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <p
      class={[
        "flex h-10 shrink-0 items-center truncate border-t",
        "border-base-content/10 px-4 text-xs text-base-content/70",
        "tabular-nums",
      ]}
    >
      {panel === "objects"
        ? m.objects_count({ count: workspace.objects.length })
        : workspace.nouns.count(workspace.tables.length)}
    </p>
  </Panel>

  <button
    type="button"
    tabindex="-1"
    aria-label={m.resize()}
    onpointerdown={startResize}
    ondblclick={() => workspace.setAsideWidth(rem(ASIDE))}
    class={[
      "absolute inset-y-0 -right-2 z-10 w-2 cursor-col-resize bg-transparent",
      "transition-colors hover:bg-primary/40",
    ]}
  ></button>
</div>
