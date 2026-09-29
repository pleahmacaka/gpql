<script lang="ts">
  import { Dialog, type Ending, Icon, Keycap } from "@gpql/ui"

  import { newDiagram, openDiagram } from "$lib/components/erd/files"
  import {
    canFormat,
    formatNow,
    formattable,
  } from "$lib/components/query/format"
  import {
    launcher,
    openDatabase,
    type View,
  } from "$lib/components/session/launcher.svelte"
  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Tab } from "$lib/types"

  type Action = {
    id: string
    label: string
    hint: string
    icon: string
    run: () => void
    again?: () => void
  }

  type Props = { onclose: () => void; onsettings: () => void }

  let { onclose, onsettings }: Props = $props()

  let term = $state("")
  let ending = $state<Ending>("cancel")
  let cursor = $state(0)
  let list = $state<HTMLDivElement | null>(null)

  const TAB_NAMES: Record<Tab, () => string> = {
    data: m.tab_data,
    query: m.tab_query,
    schema: m.tab_schema,
  }

  let actions = $derived.by(() => {
    const out: Action[] = []

    const tab = (name: Tab, icon: string) =>
      out.push({
        id: `tab:${name}`,
        label: m.action_goto({ name: TAB_NAMES[name]() }),
        hint: m.hint_view(),
        icon,
        run: async () => {
          workspace.tab = name

          if (name === "schema") {
            await workspace.loadSchema()
          }
        },
      })

    if (workspace.session) {
      tab("data", "lucide:table-2")
      tab("query", "lucide:terminal")
      tab("schema", "lucide:git-fork")

      for (const entry of workspace.connections) {
        if (entry.id === workspace.activeId) {
          continue
        }

        out.push({
          id: `switch:${entry.id}`,
          label: m.action_switch({ name: entry.label }),
          hint: m.hint_session(),
          icon: workspace.iconFor(entry.handle.kind),
          run: () => workspace.show(entry.id),
        })
      }

      const active = workspace.active

      if (active) {
        out.push({
          id: "close",
          label: m.action_close(),
          hint: m.hint_session(),
          icon: "lucide:power",
          run: () => workspace.requestClose(active.id),
        })
      }

      out.push({
        id: "add",
        label: m.session_add(),
        hint: m.hint_session(),
        icon: "lucide:plus",
        run: () => launcher.open("new"),
      })

      for (const saved of workspace.query.saved) {
        out.push({
          id: `saved:${saved.id}`,
          label: saved.name,
          hint: m.hint_saved(),
          icon: "lucide:bookmark",
          run: () => {
            workspace.tab = "query"
            workspace.query.load(saved.id)
          },
        })
      }

      if (formattable(workspace.dialect) && canFormat()) {
        out.push({
          id: "format",
          label: m.format_sql(),
          hint: m.tab_query(),
          icon: "lucide:wand-sparkles",
          run: () => {
            workspace.tab = "query"
            formatNow()
          },
        })
      }

      const query = workspace.query

      for (const entry of query.history.slice(0, 20)) {
        const sql = entry.sql

        const reopen = async () => {
          workspace.tab = "query"
          await query.replace(sql)
        }

        out.push({
          id: `history:${entry.id}`,
          label: sql.trim().split("\n")[0],
          hint: m.tab_history(),
          icon: "lucide:history",
          run: reopen,
          again: async () => {
            await reopen()
            await query.run()
          },
        })
      }

      for (const table of workspace.tables) {
        out.push({
          id: `table:${table.name}`,
          label: table.name,
          hint: workspace.nouns.row(table.rows),
          icon:
            workspace.session?.kind === "mqtt"
              ? "lucide:radio"
              : "lucide:table-2",
          run: async () => {
            workspace.tab = "data"
            await workspace.select(table.name)
          },
        })
      }
    } else {
      const mode = (name: View, label: string, icon: string) =>
        out.push({
          id: `mode:${name}`,
          label,
          hint: m.hint_connect(),
          icon,
          run: () => launcher.open(name),
        })

      mode("new", m.action_new(), "lucide:plus")
      mode("quick", m.action_quick(), "lucide:radar")
      mode("recent", m.action_recent(), "lucide:history")
    }

    for (const entry of workspace.recents) {
      if (workspace.connections.some(open => open.origin === entry.url)) {
        continue
      }

      out.push({
        id: `recent:${entry.url}`,
        label: entry.alias ?? entry.label,
        hint: entry.alias ? entry.label : entry.detail,
        icon:
          entry.kind === "erd"
            ? "lucide:git-fork"
            : workspace.iconFor(entry.kind),
        run: () => workspace.resume(entry.url, entry.kind),
      })
    }

    const flip = (
      key: "dark" | "compact" | "readOnly" | "acrylic" | "autoscan",
      label: string,
      icon: string,
    ) =>
      out.push({
        id: `toggle:${key}`,
        label,
        hint: workspace[key] ? m.hint_on() : m.hint_off(),
        icon,
        run: () =>
          key === "readOnly"
            ? workspace.setReadOnly(!workspace.readOnly)
            : workspace.toggle(key),
      })

    flip("dark", m.action_toggle({ name: m.option_dark() }), "lucide:moon")
    flip(
      "compact",
      m.action_toggle({ name: m.option_compact() }),
      "lucide:rows-3",
    )
    flip("readOnly", m.action_toggle({ name: m.read_only() }), "lucide:lock")
    flip(
      "acrylic",
      m.action_toggle({ name: m.option_acrylic() }),
      "lucide:layers",
    )

    out.push({
      id: "diagram",
      label: m.erd_new(),
      hint: m.hint_connect(),
      icon: "lucide:file-plus-2",
      run: newDiagram,
    })

    out.push({
      id: "open-diagram",
      label: m.erd_open(),
      hint: m.hint_connect(),
      icon: "lucide:folder-open",
      run: openDiagram,
    })

    out.push({
      id: "open-file",
      label: m.action_open_file(),
      hint: m.hint_connect(),
      icon: "lucide:database",
      run: openDatabase,
    })

    out.push({
      id: "scan",
      label: m.action_scan(),
      hint: m.hint_connect(),
      icon: "lucide:radar",
      run: () => workspace.scan(),
    })

    out.push({
      id: "settings",
      label: m.action_settings(),
      hint: m.hint_app(),
      icon: "lucide:settings",
      run: onsettings,
    })

    return out
  })

  let matches = $derived.by(() => {
    const needle = term.trim().toLowerCase()

    if (needle === "") {
      return actions.slice(0, 40)
    }

    return actions
      .filter(action =>
        `${action.label} ${action.hint}`.toLowerCase().includes(needle),
      )
      .slice(0, 40)
  })

  $effect(() => {
    void matches
    cursor = 0
  })

  $effect(() => {
    list
      ?.querySelector(`[data-index="${cursor}"]`)
      ?.scrollIntoView({ block: "nearest" })
  })

  let current = $derived(matches[cursor])

  function pick(action: Action | undefined, rerun = false) {
    if (!action) {
      return
    }

    ending = "confirm"
    onclose()

    if (rerun && action.again) {
      action.again()
    } else {
      action.run()
    }
  }

  function keys(event: KeyboardEvent) {
    const last = matches.length - 1
    const moves: Record<string, number> = {
      ArrowDown: Math.min(cursor + 1, last),
      ArrowUp: Math.max(cursor - 1, 0),
      PageDown: Math.min(cursor + 8, last),
      PageUp: Math.max(cursor - 8, 0),
    }

    if (event.key in moves) {
      event.preventDefault()
      cursor = moves[event.key]

      return
    }

    if (event.key === "Enter" && !event.isComposing) {
      event.preventDefault()
      pick(current, event.ctrlKey)
    }
  }
</script>

<Dialog
  label={m.action_palette()}
  {onclose}
  dismiss={m.close()}
  size="lg"
  place="top"
  {ending}
>
  <div class="flex items-center gap-3 border-b border-base-content/10 px-4">
    <Icon icon="lucide:search" class="size-4 shrink-0 text-base-content/70" />

    <input
      bind:value={term}
      onkeydown={keys}
      data-autofocus
      role="combobox"
      aria-expanded="true"
      aria-controls="palette-list"
      aria-activedescendant={current ? `palette-${cursor}` : undefined}
      aria-label={m.action_palette()}
      placeholder={m.quick_placeholder()}
      class={[
        "min-w-0 flex-1 bg-transparent py-4 text-base outline-none",
        "placeholder:text-base-content/60",
      ]}
    />
  </div>

  <div
    bind:this={list}
    id="palette-list"
    role="listbox"
    aria-label={m.action_palette()}
    class="max-h-96 overflow-y-auto p-2"
  >
    {#each matches as action, index (action.id)}
      <button
        type="button"
        id="palette-{index}"
        role="option"
        tabindex="-1"
        aria-selected={index === cursor}
        aria-keyshortcuts={action.again ? "Control+Enter" : undefined}
        data-index={index}
        onclick={event => pick(action, event.ctrlKey)}
        onmousemove={() => (cursor = index)}
        class={[
          "relative flex w-full cursor-pointer items-center gap-3 px-3 py-2",
          "text-left",
          index === cursor ? "bg-primary/10" : "hover:bg-base-content/5",
        ]}
      >
        {#if index === cursor}
          <span
            aria-hidden="true"
            class="absolute inset-y-0 left-0 w-1 bg-primary"
          ></span>
        {/if}

        <Icon
          icon={action.icon}
          class={[
            "size-4 shrink-0",
            index === cursor ? "text-primary" : "text-base-content/70",
          ]}
        />

        <span class="min-w-0 flex-1 truncate text-sm">{action.label}</span>

        <span class="max-w-48 shrink-0 truncate text-xs text-base-content/70">
          {action.hint}
        </span>
      </button>
    {:else}
      <p class="px-3 py-6 text-center text-sm text-base-content/70">
        {m.quick_empty()}
      </p>
    {/each}
  </div>

  <footer
    class={[
      "flex items-center gap-4 border-t border-base-content/10 px-4 py-2",
      "text-xs text-base-content/70",
    ]}
  >
    <span class="flex items-center gap-2">
      <Keycap keys={["↑", "↓"]} />
      {m.palette_move()}
    </span>

    <span class="flex items-center gap-2">
      <Keycap keys={["enter"]} />
      {current?.again ? m.history_open() : m.palette_run()}
    </span>

    {#if current?.again}
      <span class="flex items-center gap-2">
        <Keycap keys={["ctrl", "enter"]} />
        {m.history_run()}
      </span>
    {/if}

    <span class="flex items-center gap-2">
      <Keycap keys={["escape"]} />
      {m.close()}
    </span>
  </footer>
</Dialog>
