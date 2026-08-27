<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { onMount } from "svelte"

  import { Icon, ListRow, menu } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { SessionConfig } from "$lib/types"

  type Props = { onedit?: (config: SessionConfig) => void }

  let { onedit }: Props = $props()

  let shaking = $state<string | null>(null)
  let naming = $state<{ url: string; draft: string } | null>(null)

  onMount(() => {
    workspace.sniff()
  })

  function reason(url: string) {
    const code = workspace.unreachable[url]

    if (code === "gone") {
      return m.file_gone()
    }

    if (code === "refused") {
      return m.bad_credentials()
    }

    if (code === "forgotten") {
      return m.login_forgotten()
    }

    return m.cannot_connect()
  }

  function refuse(url: string) {
    shaking = url

    setTimeout(() => {
      if (shaking === url) {
        shaking = null
      }
    }, 400)
  }

  function pick(url: string, kind: string) {
    if (workspace.unreachable[url]) {
      refuse(url)

      return
    }

    workspace.resume(url, kind)
  }

  async function edit(url: string) {
    const config = await workspace.settings(url)

    if (config) {
      onedit?.(config)
    }
  }


  function openMenu(event: MouseEvent, url: string, label: string) {
    menu.show(event, [
      {
        label: m.connect(),
        icon: "lucide:plug",
        run: () => workspace.resume(url, "", true),
      },
      {
        label: m.menu_edit(),
        icon: "lucide:pencil",
        run: () => edit(url),
      },
      {
        label: m.menu_alias(),
        icon: "lucide:tag",
        run: () => {
          const entry = workspace.recents.find(item => item.url === url)

          naming = { url, draft: entry?.alias ?? "" }
        },
      },
      {
        label: m.menu_copy_address(),
        icon: "lucide:copy",
        run: () => navigator.clipboard.writeText(url),
      },
      {
        label: m.menu_forget({ name: label }),
        icon: "lucide:x",
        danger: true,
        run: () => workspace.forgetRecent(url),
      },
])
  }
</script>

<div class="space-y-1">
  <button
    type="button"
    onclick={() => {
      workspace.editing = null
      workspace.mode = "new"
    }}
    class="flex w-full items-center justify-center gap-1.5 rounded-field
      border border-dashed border-base-content/20 px-3 py-2 text-sm
      text-base-content/60 transition-colors hover:border-primary/50
      hover:text-primary"
  >
    <Icon icon="lucide:plus" class="size-4" />
    {m.panel_new()}
  </button>

  {#if workspace.recents.length === 0}
    <p class="px-3 py-6 text-center text-sm text-base-content/45">
      {m.recent_empty()}
    </p>
  {:else}
    {#each workspace.recents as entry (entry.url)}
      {#if naming?.url === entry.url}
        <div
          class="flex items-center gap-2 rounded-field bg-base-200 px-3 py-2"
        >
          <Icon icon="lucide:tag" class="size-4 shrink-0 text-base-content/40" />

          <!-- svelte-ignore a11y_autofocus -->
          <input
            autofocus
            value={naming.draft}
            placeholder={entry.label}
            oninput={event => {
              if (naming) {
                naming.draft = event.currentTarget.value
              }
            }}
            onblur={() => (naming = null)}
            onkeydown={event => {
              if (event.key === "Enter" && naming) {
                workspace.renameRecent(entry.url, naming.draft)
                naming = null
              }

              if (event.key === "Escape") {
                event.stopPropagation()
                naming = null
              }
            }}
            class="min-w-0 flex-1 bg-transparent text-sm outline-none
              select-text placeholder:text-base-content/30"
          />
        </div>
      {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div oncontextmenu={event => openMenu(event, entry.url, entry.label)}>
        <ListRow
        icon={entry.kind === "erd"
          ? "lucide:git-fork"
          : entry.kind === "sqlite"
            ? "lucide:file"
            : "lucide:database"}
        title={entry.alias ?? entry.label}
        detail={workspace.dialing === entry.url
          ? m.connecting_now()
          : workspace.unreachable[entry.url]
            ? reason(entry.url)
            : entry.alias
              ? `${entry.label} · ${entry.detail}`
              : entry.detail}
        tone={workspace.unreachable[entry.url] && workspace.dialing !== entry.url
          ? "bad"
          : "plain"}
        busy={workspace.dialing === entry.url}
        shaking={shaking === entry.url}
        onclick={() => pick(entry.url, entry.kind)}
        onedit={entry.kind === "erd" ? undefined : () => edit(entry.url)}
          ondismiss={() => workspace.forgetRecent(entry.url)}
        />
      </div>
      {/if}
    {/each}
  {/if}
</div>
