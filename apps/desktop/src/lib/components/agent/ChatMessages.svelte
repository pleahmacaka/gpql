<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon, menu } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  let chat = $derived(workspace.chat)

  let editing = $state<{ index: number; draft: string } | null>(null)

  function openMenu(event: MouseEvent, index: number) {
    const turn = chat.turns[index]

    menu.show(event, [
      {
        label: m.menu_copy(),
        icon: "lucide:copy",
        run: () => navigator.clipboard.writeText(turn.text),
      },
      ...(turn.role === "you"
        ? [
            {
              label: m.menu_edit_message(),
              icon: "lucide:pencil",
              run: () => {
                editing = { index, draft: turn.text }
              },
            },
          ]
        : []),
      {
        label: m.menu_delete(),
        icon: "lucide:x",
        danger: true,
        run: () => chat.deleteTurn(index),
      },
    ])
  }

  function commit() {
    if (editing) {
      const { index, draft } = editing

      editing = null
      void chat.editTurn(index, draft)
    }
  }
</script>

{#each chat.turns as turn, index (turn.id)}
  {#if editing?.index === index}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      autofocus
      value={editing.draft}
      oninput={event => {
        if (editing) {
          editing.draft = event.currentTarget.value
        }
      }}
      onkeydown={event => {
        if (event.key === "Enter") {
          event.preventDefault()
          commit()
        }

        if (event.key === "Escape") {
          event.stopPropagation()
          editing = null
        }
      }}
      onblur={() => (editing = null)}
      class="w-full rounded-field bg-base-200 px-3 py-2 text-xs
        outline-none select-text ring-1 ring-primary/40"
    />
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      oncontextmenu={event => openMenu(event, index)}
      class="max-w-4/5 rounded-field px-3 py-2 text-xs
        whitespace-pre-wrap {turn.role === 'you'
        ? 'self-end bg-primary/10 text-primary'
        : 'self-start bg-base-200'}"
    >
      {turn.text}
    </div>

    {#if chat.branch?.at === index && chat.branch.threads.length > 1}
      {@const alts = chat.branch}

      <div
        class="flex items-center gap-1 self-end text-xs text-base-content/40"
      >
        <button
          type="button"
          aria-label="previous version"
          onclick={() => chat.pickThread(alts.pick - 1)}
          disabled={alts.pick === 0}
          class="rounded-selector p-1 hover:text-base-content
            disabled:opacity-30"
        >
          <Icon icon="lucide:chevron-left" class="size-3" />
        </button>

        {alts.pick + 1}/{alts.threads.length}

        <button
          type="button"
          aria-label="next version"
          onclick={() => chat.pickThread(alts.pick + 1)}
          disabled={alts.pick === alts.threads.length - 1}
          class="rounded-selector p-1 hover:text-base-content
            disabled:opacity-30"
        >
          <Icon icon="lucide:chevron-right" class="size-3" />
        </button>
      </div>
    {/if}
  {/if}
{/each}

{#if chat.error}
  <p class="self-start px-1 text-xs text-error">{chat.error}</p>
{/if}

