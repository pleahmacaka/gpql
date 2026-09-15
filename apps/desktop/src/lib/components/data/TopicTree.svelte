<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { createVirtualizer } from "@tanstack/svelte-virtual"
  import { flip } from "svelte/animate"
  import { untrack } from "svelte"
  import { fade, scale } from "svelte/transition"

  import { Icon, calm, menu, pop, rem, veil } from "@gpql/ui"
  import { FORMATS } from "$lib/session/exporting"
  import { workspace } from "$lib/session/workspace.svelte"

  import type { ExportFormat } from "$lib/types"

  type Props = {
    query: string
    onexport: (table: string, format: ExportFormat) => void
  }

  let { query, onexport }: Props = $props()

  type Node = {
    segment: string
    path: string
    children: Map<string, Node>
    count: number | null
  }

  type Flat = {
    path: string
    label: string
    depth: number
    folder: boolean
    count: number
    inside: number
  }

  let open = $state<Record<string, boolean>>({})
  let filter = $state("")
  let scroller = $state<HTMLDivElement | null>(null)

  let root = $derived.by(() => {
    const needle = query.trim().toLowerCase()
    const head: Node = { segment: "", path: "", children: new Map(), count: null }

    for (const topic of workspace.tables) {
      if (needle !== "" && !topic.name.toLowerCase().includes(needle)) {
        continue
      }

      let node = head
      let path = ""

      for (const segment of topic.name.split("/")) {
        path = path === "" ? segment : `${path}/${segment}`

        if (!node.children.has(segment)) {
          node.children.set(segment, {
            segment,
            path,
            children: new Map(),
            count: null,
          })
        }

        node = node.children.get(segment) as Node
      }

      node.count = topic.rows
    }

    return head
  })

  function widen(node: Node, depth: number, out: Flat[]) {
    const ordered = [...node.children.values()].sort((a, b) =>
      a.path.localeCompare(b.path),
    )

    for (const child of ordered) {
      const folder = child.children.size > 0
      const inside = folder ? countInside(child) : 0

      out.push({
        path: child.path,
        label: child.segment === "" ? "/" : child.segment,
        depth,
        folder,
        count: child.count ?? 0,
        inside,
      })

      const shut = open[child.path] ?? child.path.startsWith("$SYS")

      if (folder && !shut) {
        widen(child, depth + 1, out)
      }
    }
  }

  function countInside(node: Node): number {
    let found = node.count === null ? 0 : 1

    for (const child of node.children.values()) {
      found += countInside(child)
    }

    return found
  }

  let flat = $derived.by(() => {
    const out: Flat[] = []

    widen(root, 0, out)

    return out
  })

  const rows = createVirtualizer<HTMLDivElement, HTMLButtonElement>({
    count: 0,
    getScrollElement: () => scroller,
    estimateSize: () => rem(2),
    overscan: 10,
  })

  $effect(() => {
    const count = flat.length
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

  let subs = $derived(workspace.active?.subs ?? [])

  async function addFilter() {
    const next = filter.trim()

    if (next === "" || !workspace.active) {
      return
    }

    await workspace.active.mqttSubscribe(next, 1)
    filter = ""
  }

  function openMenu(event: MouseEvent, entry: Flat) {
    const topic = entry.path

    menu.show(event, [
      {
        label: m.mqtt_publish_to(),
        icon: "lucide:send",
        run: async () => {
          workspace.tab = "data"

          if (workspace.active) {
            workspace.active.draft = { ...workspace.active.draft, topic }
          }

          await workspace.select(topic)
        },
      },
      {
        label: m.mqtt_clear(),
        icon: "lucide:trash-2",
        run: () => workspace.active?.mqttClear(topic),
      },
      {
        label: workspace.favorites.includes(topic)
          ? m.menu_unfavorite()
          : m.menu_favorite(),
        icon: "lucide:star",
        run: () => workspace.toggleFavorite(topic),
      },
      {
        label: m.menu_copy_name(),
        icon: "lucide:copy",
        run: () => navigator.clipboard.writeText(topic),
      },
      ...FORMATS.map(format => ({
        label: m.menu_export_as({ format: format.toUpperCase() }),
        icon: "lucide:download",
        run: () => onexport(topic, format),
      })),
    ])
  }
</script>

<div class="flex flex-col gap-1 px-2 pb-1">
  <span class="px-1 text-xs text-base-content/40">{m.mqtt_filters()}</span>

  <div class="flex min-h-6 flex-wrap items-center gap-1 px-1">
    {#each subs as sub (sub.filter)}
      <span
        animate:flip={{ duration: calm() ? 0 : 150 }}
        transition:scale|local={pop()}
        class="flex items-center gap-1 rounded-selector bg-base-200 px-2 py-1
          text-xs"
      >
        {sub.filter}
        <span class="text-base-content/35">qos{sub.qos}</span>

        {#if subs.length > 1}
          <button
            type="button"
            aria-label={m.close()}
            onclick={() => workspace.active?.mqttUnsubscribe(sub.filter)}
            class="text-base-content/35 hover:text-base-content"
          >
            <Icon icon="lucide:x" class="size-3" />
          </button>
        {/if}
      </span>
    {/each}
  </div>

  <div class="flex gap-1 px-1">
    <input
      bind:value={filter}
      placeholder={m.mqtt_add_filter()}
      onkeydown={event => event.key === "Enter" && addFilter()}
      class="min-w-0 flex-1 rounded-field bg-base-200 px-2 py-1 text-xs
        outline-none select-text placeholder:text-base-content/30"
    />

    <button
      type="button"
      onclick={addFilter}
      class="rounded-field bg-base-200 px-2 py-1 text-xs hover:bg-base-300"
    >
      <Icon icon="lucide:plus" class="size-4" />
    </button>
  </div>
</div>

<div
  bind:this={scroller}
  class="flex-1 scroll-smooth overflow-y-auto px-2"
  style:scrollbar-gutter="stable"
>
  {#if flat.length === 0}
    <p in:fade|local={veil()} class="px-2 py-4 text-xs text-base-content/35">
      {workspace.nouns.none()}
    </p>
  {/if}

  <div class="relative" style:height="{$rows.getTotalSize()}px">
    {#each $rows.getVirtualItems() as row (row.key)}
      {@const entry = flat[row.index]}

      {#if entry}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          oncontextmenu={event => !entry.folder && openMenu(event, entry)}
          class="group absolute inset-x-0 flex items-center gap-2 rounded-field
            px-2 contain-paint transition-colors {!entry.folder &&
          workspace.browse.table === entry.path
            ? 'bg-primary/10 text-primary'
            : 'hover:bg-base-200'}"
          style:height="{row.size}px"
          style:transform="translateY({row.start}px)"
        >
          {#if entry.folder}
            <button
              type="button"
              aria-expanded={!(open[entry.path] ?? entry.path.startsWith("$SYS"))}
              onclick={() =>
                (open = {
                  ...open,
                  [entry.path]: !(open[entry.path] ??
                    entry.path.startsWith("$SYS")),
                })}
              class="flex min-w-0 flex-1 items-center gap-2 text-left"
              style:margin-left="{entry.depth * 0.75}rem"
            >
              <Icon
                icon={open[entry.path] ?? entry.path.startsWith("$SYS")
                  ? "lucide:chevron-right"
                  : "lucide:chevron-down"}
                class="size-3 shrink-0 opacity-60"
              />

              <span class="truncate text-sm text-base-content/60" title={entry.path}>
                {entry.label}
              </span>

              <span class="shrink-0 text-xs text-base-content/30">
                {entry.inside}
              </span>
            </button>
          {:else}
            <button
              type="button"
              onclick={() => workspace.select(entry.path)}
              aria-pressed={workspace.browse.table === entry.path}
              class="flex min-w-0 flex-1 items-center gap-2 text-left"
              style:margin-left="{entry.depth * 0.75 + 0.75}rem"
            >
              <Icon icon="lucide:radio" class="size-4 shrink-0 opacity-60" />

              <span class="truncate text-sm" title={entry.path}>
                {entry.label}
              </span>

              {#if entry.count > 0}
                <span class="shrink-0 text-xs text-base-content/35">
                  {entry.count}
                </span>
              {/if}
            </button>
          {/if}
        </div>
      {/if}
    {/each}
  </div>
</div>
