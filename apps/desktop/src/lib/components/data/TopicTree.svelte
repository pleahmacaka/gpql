<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { createVirtualizer } from "@tanstack/svelte-virtual"
  import { untrack } from "svelte"
  import { flip } from "svelte/animate"
  import { scale } from "svelte/transition"

  import {
    contextmenu,
    Icon,
    type MenuItem,
    pop,
    rem,
    TIMING,
    tooltip,
  } from "@gpql/ui"
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
    topic: boolean
    count: number
    inside: number
  }

  const unit = rem(1)

  let open = $state<Record<string, boolean>>({})
  let filter = $state("")
  let scroller = $state<HTMLDivElement | null>(null)

  let root = $derived.by(() => {
    const needle = query.trim().toLowerCase()
    const head: Node = {
      segment: "",
      path: "",
      children: new Map(),
      count: null,
    }

    for (const topic of workspace.tables) {
      if (needle !== "" && !topic.name.toLowerCase().includes(needle)) {
        continue
      }

      let node = head
      let path = ""

      for (const segment of topic.name.split("/")) {
        path = path === "" ? segment : `${path}/${segment}`

        let child = node.children.get(segment)

        if (!child) {
          child = { segment, path, children: new Map(), count: null }
          node.children.set(segment, child)
        }

        node = child
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
        topic: child.count !== null,
        count: child.count ?? 0,
        inside,
      })

      if (folder && !shut(child.path)) {
        widen(child, depth + 1, out)
      }
    }
  }

  function shut(path: string) {
    return open[path] ?? path.startsWith("$SYS")
  }

  function fold(path: string) {
    open = { ...open, [path]: !shut(path) }
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

  const rows = createVirtualizer<HTMLDivElement, HTMLDivElement>({
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

  function itemsFor(entry: Flat): MenuItem[] {
    const topic = entry.path
    const copyName = {
      label: m.menu_copy_name(),
      icon: "lucide:copy",
      run: () => navigator.clipboard.writeText(topic),
    }

    if (!entry.topic) {
      return [copyName]
    }

    return [
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
      copyName,
      ...FORMATS.map(format => ({
        label: m.menu_export_as({ format: format.toUpperCase() }),
        icon: "lucide:download",
        run: () => onexport(topic, format),
      })),
    ]
  }
</script>

<section
  aria-label={m.mqtt_filters()}
  class="flex shrink-0 flex-col gap-2 border-b border-base-content/10 p-3"
>
  <h3 class="text-xs font-medium text-base-content/70">{m.mqtt_filters()}</h3>

  {#if subs.length > 0}
    <ul class="flex flex-wrap gap-1">
      {#each subs as sub (sub.filter)}
        <li
          animate:flip={{ duration: TIMING.quick }}
          transition:scale|local={pop()}
          class="flex items-center gap-1 bg-base-200 py-1 pl-2 text-xs hairline"
        >
          <span class="max-w-40 truncate">{sub.filter}</span>
          <span class="text-base-content/70 tabular-nums">
            {m.mqtt_qos({ level: String(sub.qos) })}
          </span>

          {#if subs.length > 1}
            <button
              type="button"
              aria-label="{m.mqtt_unsubscribe()} {sub.filter}"
              use:tooltip={m.mqtt_unsubscribe()}
              onclick={() => workspace.active?.mqttUnsubscribe(sub.filter)}
              class={[
                "grid size-4 cursor-pointer place-items-center",
                "text-base-content/60 hover:text-error",
              ]}
            >
              <Icon icon="lucide:x" class="size-3" />
            </button>
          {:else}
            <span class="w-1"></span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <div class="flex gap-2">
    <input
      bind:value={filter}
      placeholder={m.mqtt_add_filter()}
      aria-label={m.mqtt_subscribe()}
      spellcheck="false"
      onkeydown={event => {
        if (event.key === "Enter" && !event.isComposing) {
          event.preventDefault()
          void addFilter()
        }
      }}
      class={[
        "input input-sm min-w-0 flex-1 bg-base-100 select-text",
        "placeholder:text-base-content/60",
      ]}
    />

    <button
      type="button"
      aria-label={m.mqtt_subscribe()}
      use:tooltip={m.mqtt_subscribe()}
      disabled={filter.trim() === ""}
      onclick={addFilter}
      class="btn btn-square btn-soft btn-sm"
    >
      <Icon icon="lucide:plus" class="size-4" />
    </button>
  </div>
</section>

<div
  bind:this={scroller}
  class="min-h-0 flex-1 overflow-y-auto py-1"
  style:scrollbar-gutter="stable"
>
  {#if flat.length === 0}
    <p class="px-4 py-6 text-center text-xs text-base-content/70">
      {query.trim() === "" ? workspace.nouns.none() : m.no_match()}
    </p>
  {/if}

  <div class="relative" style:height="{$rows.getTotalSize() / unit}rem">
    {#each $rows.getVirtualItems() as row (row.key)}
      {@const entry = flat[row.index]}

      {#if entry}
        {@const picked = entry.topic && workspace.browse.table === entry.path}

        <div
          use:contextmenu={() => itemsFor(entry)}
          class={[
            "group absolute inset-x-0 flex items-center pr-3 pl-3",
            "contain-paint transition-colors",
            picked ? "bg-primary/10 text-primary" : "hover:bg-base-content/5",
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

          {#each { length: entry.depth }, level (level)}
            <span aria-hidden="true" class="relative h-full w-4 shrink-0">
              <span
                class={[
                  "absolute inset-y-0 left-2 border-l",
                  "border-base-content/10",
                ]}
              ></span>
            </span>
          {/each}

          {#if entry.folder}
            <button
              type="button"
              aria-label={entry.path}
              aria-expanded={!shut(entry.path)}
              onclick={() => fold(entry.path)}
              class={[
                "grid size-4 shrink-0 cursor-pointer place-items-center",
                "text-base-content/60 hover:text-base-content",
              ]}
            >
              <Icon
                icon="lucide:chevron-right"
                class={[
                  "size-3 transition-transform",
                  !shut(entry.path) && "rotate-90",
                ]}
              />
            </button>
          {:else}
            <span class="size-4 shrink-0"></span>
          {/if}

          <button
            type="button"
            onclick={() =>
              entry.topic ? workspace.select(entry.path) : fold(entry.path)}
            aria-current={picked ? "true" : undefined}
            class={[
              "flex h-full min-w-0 flex-1 cursor-pointer items-center gap-2",
              "pl-1 text-left outline-offset-0",
            ]}
          >
            {#if entry.topic}
              <Icon
                icon="lucide:radio"
                class={[
                  "size-4 shrink-0",
                  picked ? "text-primary" : "text-base-content/60",
                ]}
              />
            {/if}

            <span
              class={[
                "truncate text-sm",
                !entry.topic && "text-base-content/70",
                picked && "font-medium",
              ]}
              title={entry.path}
            >
              {entry.label}
            </span>
          </button>

          {#if entry.folder || entry.count > 0}
            <span
              class={[
                "shrink-0 pl-2 text-xs tabular-nums",
                picked ? "text-primary" : "text-base-content/70",
              ]}
            >
              {entry.folder ? entry.inside : entry.count}
            </span>
          {/if}
        </div>
      {/if}
    {/each}
  </div>
</div>
