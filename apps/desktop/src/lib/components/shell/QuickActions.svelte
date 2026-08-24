<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon } from "@gpql/ui"

  import { workspace } from "$lib/session/workspace.svelte"
  import type { Mode, Tab } from "$lib/types"

  type Action = {
    id: string
    label: string
    hint: string
    icon: string
    run: () => void
  }

  type Props = { onclose: () => void; onsettings: () => void }

  let { onclose, onsettings }: Props = $props()

  let term = $state("")
  let cursor = $state(0)

  let actions = $derived.by(() => {
    const out: Action[] = []

    const tab = (name: Tab, icon: string) =>
      out.push({
        id: `tab:${name}`,
        label: m.action_goto({ name }),
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

      out.push({
        id: "close",
        label: m.action_close(),
        hint: m.hint_session(),
        icon: "lucide:power",
        run: () => workspace.close(),
      })

      for (const table of workspace.tables) {
        out.push({
          id: `table:${table.name}`,
          label: table.name,
          hint: m.rows_count({ count: table.rows }),
          icon: "lucide:table-2",
          run: async () => {
            workspace.tab = "data"
            await workspace.select(table.name)
          },
        })
      }
    } else {
      const mode = (name: Mode, label: string, icon: string) =>
        out.push({
          id: `mode:${name}`,
          label,
          hint: m.hint_connect(),
          icon,
          run: () => {
            workspace.mode = name
          },
        })

      mode("new", m.action_new(), "lucide:plus")
      mode("quick", m.action_quick(), "lucide:radar")
      mode("recent", m.action_recent(), "lucide:history")
    }

    for (const entry of workspace.recents) {
      out.push({
        id: `recent:${entry.url}`,
        label: entry.label,
        hint: entry.detail,
        icon: entry.kind === "sqlite" ? "lucide:file" : "lucide:database",
        run: () => workspace.resume(entry.url),
      })
    }

    for (const saved of workspace.saved) {
      out.push({
        id: `saved:${saved.id}`,
        label: saved.name,
        hint: m.hint_saved(),
        icon: "lucide:bookmark",
        run: () => {
          workspace.tab = "query"
          workspace.sql = saved.sql
        },
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
        run: () => workspace.toggle(key),
      })

    flip("dark", m.action_toggle({ name: m.option_dark() }), "lucide:moon")
    flip("compact", m.action_toggle({ name: m.option_compact() }), "lucide:rows-3")
    flip("readOnly", m.action_toggle({ name: m.read_only() }), "lucide:lock")
    flip("acrylic", m.action_toggle({ name: m.option_acrylic() }), "lucide:layers")

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
      return actions.slice(0, 12)
    }

    return actions
      .filter(action =>
        `${action.label} ${action.hint}`.toLowerCase().includes(needle),
      )
      .slice(0, 12)
  })

  $effect(() => {
    void matches
    cursor = 0
  })

  function pick(action: Action | undefined) {
    if (!action) {
      return
    }

    onclose()
    action.run()
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault()
      cursor = Math.min(cursor + 1, matches.length - 1)
    }

    if (event.key === "ArrowUp") {
      event.preventDefault()
      cursor = Math.max(cursor - 1, 0)
    }

    if (event.key === "Enter") {
      event.preventDefault()
      pick(matches[cursor])
    }
  }
</script>

<div
  class="fixed inset-0 z-60 scrim"
  role="presentation"
  onclick={onclose}
></div>

<div
  class="fixed inset-x-0 top-24 z-60 mx-auto w-lg max-w-11/12 overflow-hidden
    rounded-box bg-base-100 lift"
  role="dialog"
  aria-label="Quick actions"
  tabindex="-1"
>
  <div class="flex items-center gap-2 px-4 py-3">
    <Icon icon="lucide:search" class="size-4 shrink-0 text-base-content/40" />

    <!-- svelte-ignore a11y_autofocus -->
    <input
      autofocus
      bind:value={term}
      onkeydown={keys}
      placeholder={m.quick_placeholder()}
      class="min-w-0 flex-1 bg-transparent text-sm outline-none
        placeholder:text-base-content/30"
    />

    <kbd class="text-xs text-base-content/30">esc</kbd>
  </div>

  <div class="max-h-80 overflow-y-auto px-2 pb-2">
    {#each matches as action, index (action.id)}
      <button
        type="button"
        onclick={() => pick(action)}
        onmouseenter={() => (cursor = index)}
        class="flex w-full items-center gap-3 rounded-field px-2 py-2 text-left
          {index === cursor ? 'bg-primary/10 text-primary' : ''}"
      >
        <Icon icon={action.icon} class="size-4 shrink-0 opacity-60" />

        <span class="min-w-0 flex-1 truncate text-sm">{action.label}</span>

        <span class="shrink-0 text-xs text-base-content/40">{action.hint}</span>
      </button>
    {:else}
      <p class="px-2 py-4 text-sm text-base-content/45">{m.quick_empty()}</p>
    {/each}
  </div>
</div>
