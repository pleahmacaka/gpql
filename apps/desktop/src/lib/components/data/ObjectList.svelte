<script lang="ts">
  import { slide } from "svelte/transition"

  import { contextmenu, Icon, type MenuItem, TIMING } from "@gpql/ui"

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

  const READABLE: ObjectKind[] = ["view", "sequence"]

  async function show(entry: DbObject) {
    if (!READABLE.includes(entry.kind)) {
      await workspace.showDdl(entry.name, entry.kind, entry.detail)

      return
    }

    workspace.tab = "data"
    await workspace.select(entry.name)
  }

  function itemsFor(entry: DbObject): MenuItem[] {
    return [
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
          ]
        : []),
      {
        label: m.menu_ddl(),
        icon: "lucide:file-code-2",
        run: () => workspace.showDdl(entry.name, entry.kind, entry.detail),
      },
      {
        label: m.menu_copy_name(),
        icon: "lucide:copy",
        run: () => navigator.clipboard.writeText(entry.name),
      },
    ]
  }
</script>

{#each grouped as [kind, entries] (kind)}
  {@const shut = open[kind] === false}

  <button
    type="button"
    aria-expanded={!shut}
    onclick={() => (open = { ...open, [kind]: shut })}
    class={[
      "flex w-full cursor-pointer items-center gap-2 px-4 pt-3 pb-1 text-xs",
      "font-medium text-base-content/70 transition-colors",
      "hover:text-base-content",
    ]}
  >
    <Icon
      icon="lucide:chevron-right"
      class={["size-3 shrink-0 transition-transform", !shut && "rotate-90"]}
    />
    <span class="flex-1 text-left">{HEADINGS[kind]()}</span>
    <span class="tabular-nums">{entries.length}</span>
  </button>

  {#if !shut}
    <ul transition:slide|local={{ duration: TIMING.quick }}>
      {#each entries as entry, index (index)}
        {@const picked =
          workspace.browse.table === entry.name ||
          workspace.ddl?.name === entry.name}

        <li
          use:contextmenu={() => itemsFor(entry)}
          class={[
            "relative transition-colors",
            picked ? "bg-primary/10 text-primary" : "hover:bg-base-content/5",
          ]}
        >
          {#if picked}
            <span
              aria-hidden="true"
              class="absolute inset-y-0 left-0 w-1 bg-primary"
            ></span>
          {/if}

          <button
            type="button"
            onclick={() => show(entry)}
            ondblclick={() =>
              entry.kind === "view"
                ? workspace.showDdl(entry.name, entry.kind, entry.detail)
                : undefined}
            aria-current={picked ? "true" : undefined}
            class={[
              "flex w-full cursor-pointer items-center gap-2 py-2 pr-3 pl-4",
              "text-left outline-offset-0",
            ]}
          >
            <Icon
              icon={ICONS[entry.kind]}
              class={[
                "size-4 shrink-0",
                picked ? "text-primary" : "text-base-content/60",
              ]}
            />

            <span class="min-w-0 flex-1 truncate text-sm" title={entry.name}>
              {entry.name}
            </span>

            {#if entry.detail}
              <span
                class={[
                  "max-w-24 shrink-0 truncate text-xs",
                  picked ? "text-primary" : "text-base-content/70",
                ]}
              >
                {entry.detail}
              </span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
{/each}

{#if grouped.length === 0}
  <p class="px-4 py-6 text-center text-xs text-base-content/70">
    {m.objects_none()}
  </p>
{/if}
