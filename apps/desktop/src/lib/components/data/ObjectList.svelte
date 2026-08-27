<script lang="ts">
  import { Icon, menu } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { DbObject, ObjectKind } from "$lib/types"

  type Props = { query: string }

  let { query }: Props = $props()

  const ICONS: Record<ObjectKind, string> = {
    view: "lucide:eye",
    index: "lucide:list-tree",
    sequence: "lucide:hash",
    routine: "lucide:square-function",
    trigger: "lucide:zap",
    type: "lucide:shapes",
  }

  const HEADINGS: Record<ObjectKind, () => string> = {
    view: m.kind_views,
    index: m.kind_indexes,
    sequence: m.kind_sequences,
    routine: m.kind_routines,
    trigger: m.kind_triggers,
    type: m.kind_types,
  }

  let matched = $derived.by(() => {
    const needle = query.trim().toLowerCase()

    return needle === ""
      ? workspace.objects
      : workspace.objects.filter(entry =>
          entry.name.toLowerCase().includes(needle),
        )
  })

  // group once rather than filtering the whole list per heading
  let grouped = $derived.by(() => {
    const out = new Map<ObjectKind, DbObject[]>()

    for (const entry of matched) {
      const held = out.get(entry.kind)

      if (held) {
        held.push(entry)
      } else {
        out.set(entry.kind, [entry])
      }
    }

    return [...out]
  })

  let open = $state<Record<string, boolean>>({})

  // only a view has a definition worth opening on its own
  function openMenu(event: MouseEvent, entry: DbObject) {
    menu.show(event, [
      ...(entry.kind === "view"
        ? [
            {
              label: m.tab_data(),
              icon: "lucide:table-2",
              run: async () => {
                workspace.tab = "data"
                await workspace.select(entry.name)
              },
            },
            {
              label: m.menu_ddl(),
              icon: "lucide:file-code-2",
              run: () => workspace.showDdl(entry.name),
            },
          ]
        : []),
      {
        label: m.menu_copy_name(),
        icon: "lucide:copy",
        run: () => navigator.clipboard.writeText(entry.name),
      },
    ])
  }
</script>

{#each grouped as [kind, entries] (kind)}
  {@const shut = open[kind] === false}

  <button
    type="button"
    aria-expanded={!shut}
    onclick={() => (open = { ...open, [kind]: shut })}
    class="flex w-full items-center gap-1.5 px-3 pt-2 pb-1 text-xs
      text-base-content/40 hover:text-base-content/70"
  >
    <Icon
      icon={shut ? "lucide:chevron-right" : "lucide:chevron-down"}
      class="size-3 shrink-0"
    />
    <span class="flex-1 text-left">{HEADINGS[kind]()}</span>
    <span>{entries.length}</span>
  </button>

  {#if !shut}
    {#each entries as entry (entry.kind + entry.name)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div oncontextmenu={event => openMenu(event, entry)}>
        <button
          type="button"
          ondblclick={() =>
            entry.kind === "view" ? workspace.showDdl(entry.name) : undefined}
          class="flex w-full items-center gap-2 rounded-field px-3 py-1
            text-left hover:bg-base-200"
        >
          <Icon
            icon={ICONS[entry.kind]}
            class="size-3.5 shrink-0 opacity-60"
          />

          <span class="min-w-0 flex-1 truncate text-sm" title={entry.name}>
            {entry.name}
          </span>

          {#if entry.detail}
            <span class="shrink-0 truncate text-xs text-base-content/35">
              {entry.detail}
            </span>
          {/if}
        </button>
      </div>
    {/each}
  {/if}
{/each}

{#if grouped.length === 0}
  <p class="px-3 py-4 text-center text-xs text-base-content/35">
    {m.objects_none()}
  </p>
{/if}
