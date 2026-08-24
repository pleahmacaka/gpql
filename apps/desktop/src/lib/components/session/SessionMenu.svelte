<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon } from "@gpql/ui"
  import { ListRow as ListRow } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  async function resume(url: string) {
    onclose()
    await workspace.resume(url)
  }

  async function fresh() {
    onclose()
    await workspace.close()
  }
</script>

<div
  class="fixed inset-0 z-30"
  role="presentation"
  onclick={onclose}
  onkeydown={event => event.key === "Escape" && onclose()}
></div>

<div
  class="absolute top-10 left-16 z-40 w-96 rounded-box bg-base-100 p-3 lift"
  role="menu"
  tabindex="-1"
>
  <h2 class="px-1 pb-1 text-xs text-base-content/45">{m.opened_before()}</h2>

  {#if workspace.recents.length === 0}
    <p class="px-1 py-3 text-center text-sm text-base-content/45">
      {m.recent_empty()}
    </p>
  {:else}
    {#each workspace.recents as entry (entry.url)}
      <ListRow
        icon={entry.kind === "sqlite" ? "lucide:file" : "lucide:database"}
        title={entry.label}
        detail={entry.detail}
        onclick={() => resume(entry.url)}
        ondismiss={() => workspace.forgetRecent(entry.url)}
      />
    {/each}
  {/if}

  <div class="mt-2 border-t border-base-content/8 pt-2">
    <button
      type="button"
      onclick={fresh}
      class="flex w-full items-center gap-2 rounded-field px-3 py-2 text-sm
        text-primary hover:bg-base-200"
    >
      <Icon icon="lucide:plus" class="size-4" />
      New session
    </button>
  </div>
</div>
