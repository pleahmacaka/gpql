<script lang="ts">
  import { Icon, menu } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  function openMenu(event: MouseEvent, id: string) {
    menu.show(event, [
      {
        label: m.menu_close(),
        icon: "lucide:x",
        danger: true,
        run: () => workspace.close(id),
      },
      {
        label: m.menu_close_others(),
        icon: "lucide:x",
        run: async () => {
          for (const entry of [...workspace.connections]) {
            if (entry.id !== id) {
              await workspace.close(entry.id)
            }
          }
        },
      },
    ])
  }
</script>

{#if workspace.connections.length > 0 && !workspace.erd}
  <div class="flex items-center gap-1 px-2 pb-1">
    {#each workspace.connections as entry (entry.id)}
      {@const here = entry.id === workspace.active?.id}

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        oncontextmenu={event => openMenu(event, entry.id)}
        class="group flex min-w-0 items-center rounded-field {here
          ? 'bg-primary/10 text-primary'
          : 'bg-base-200 hover:bg-base-300'}"
      >
        <button
          type="button"
          onclick={() => workspace.show(entry.id)}
          aria-pressed={here}
          class="flex min-w-0 items-center gap-1.5 px-2 py-1 text-xs"
          title={entry.handle.detail}
        >
          <Icon
            icon={workspace.iconFor(entry.handle.kind)}
            class="size-3 shrink-0"
          />

          <span class="max-w-32 truncate">{entry.label}</span>

          {#if entry.writes.open}
            <span
              class="size-1.5 shrink-0 rounded-full bg-warning"
              title={m.tx_open()}
            ></span>
          {/if}
        </button>

        <button
          type="button"
          aria-label={m.menu_close()}
          onclick={() => workspace.close(entry.id)}
          class="mr-1 rounded-selector p-0.5 text-base-content/30 opacity-0
            group-hover:opacity-100 hover:text-error focus-visible:opacity-100"
        >
          <Icon icon="lucide:x" class="size-3" />
        </button>
      </div>
    {/each}

    <button
      type="button"
      aria-label={m.session_add()}
      onclick={() => (workspace.adding = true)}
      class="rounded-field bg-base-200 px-1.5 py-1 text-base-content/45
        hover:bg-base-300 hover:text-base-content"
    >
      <Icon icon="lucide:plus" class="size-3" />
    </button>
  </div>
{/if}
