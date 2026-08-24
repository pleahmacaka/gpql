<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { ContextMenu, ListRow } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  let menu = $state<{
    x: number
    y: number
    items: { label: string; icon?: string; danger?: boolean; run: () => void }[]
  } | null>(null)

  function openMenu(event: MouseEvent, url: string, label: string) {
    event.preventDefault()

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        {
          label: m.connect(),
          icon: "lucide:plug",
          run: () => workspace.resume(url),
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
      ],
    }
  }
</script>

<div class="space-y-1">
  {#if workspace.recents.length === 0}
    <p class="px-3 py-6 text-sm text-base-content/45">
      {m.recent_empty()}
    </p>
  {:else}
    {#each workspace.recents as entry (entry.url)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div oncontextmenu={event => openMenu(event, entry.url, entry.label)}>
        <ListRow
        icon={entry.kind === "sqlite" ? "lucide:file" : "lucide:database"}
        title={entry.label}
        detail={entry.detail}
        onclick={() => workspace.resume(entry.url)}
          ondismiss={() => workspace.forgetRecent(entry.url)}
        />
      </div>
    {/each}
  {/if}

  {#if menu}
    <ContextMenu
      x={menu.x}
      y={menu.y}
      items={menu.items}
      onclose={() => (menu = null)}
    />
  {/if}
</div>
