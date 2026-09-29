<script lang="ts">
  import { contextmenu, Icon, Logo, type MenuItem } from "@gpql/ui"

  import type { ChatMove } from "$lib/ai/chat.svelte"
  import { menuBelow } from "$lib/components/query/anchor"
  import Code from "$lib/components/query/Code.svelte"
  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  let chat = $derived(workspace.chat)

  let editing = $state<{ index: number; draft: string } | null>(null)

  function actions(index: number): MenuItem[] {
    const turn = chat.turns[index]

    return [
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
        icon: "lucide:trash-2",
        danger: true,
        run: () => chat.deleteTurn(index),
      },
    ]
  }

  function commit() {
    if (editing) {
      const { index, draft } = editing

      editing = null
      void chat.editTurn(index, draft)
    }
  }

  async function openTable(table: string) {
    workspace.tab = "data"
    await workspace.select(table)
  }

  async function openQuery(sql: string, run: boolean) {
    workspace.tab = "query"
    await workspace.query.replace(sql)

    if (run) {
      await workspace.query.run()
    }
  }

  function focus(node: HTMLTextAreaElement) {
    node.focus()
    node.setSelectionRange(node.value.length, node.value.length)
  }

  function keys(event: KeyboardEvent) {
    if (event.isComposing) {
      return
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault()
      commit()
    }

    if (event.key === "Escape") {
      event.preventDefault()
      event.stopPropagation()
      editing = null
    }
  }
</script>

{#snippet moved(move: ChatMove)}
  <div class="flex flex-col bg-base-100 hairline">
    <div
      class={[
        "flex h-8 items-center gap-2 pr-1 pl-2 text-xs text-base-content/70",
        move.kind === "query" && "border-b border-base-content/10",
      ]}
    >
      <Icon
        icon={move.kind === "query" ? "lucide:terminal" : "lucide:table-2"}
        class="size-4 shrink-0"
      />

      <span class="min-w-0 flex-1 truncate">
        {move.kind === "query" ? m.tab_query() : move.table}
      </span>

      {#if move.kind === "query"}
        <button
          type="button"
          onclick={() => openQuery(move.sql, false)}
          class="btn btn-ghost btn-xs font-medium"
        >
          {m.history_open()}
        </button>

        <button
          type="button"
          onclick={() => openQuery(move.sql, true)}
          disabled={workspace.query.busy}
          class="btn btn-soft btn-primary btn-xs font-medium"
        >
          <Icon icon="lucide:play" class="size-4" />
          {m.menu_run()}
        </button>
      {:else}
        <button
          type="button"
          aria-label="{m.chat_move_open()}, {move.table}"
          onclick={() => openTable(move.table)}
          class="btn btn-ghost btn-xs font-medium"
        >
          {m.chat_move_open()}
        </button>
      {/if}
    </div>

    {#if move.kind === "query"}
      <Code
        code={move.sql}
        class="max-h-40 overflow-y-auto px-3 py-2 text-xs leading-5"
      />
    {/if}
  </div>
{/snippet}

{#each chat.turns as turn, index (turn.id)}
  {#if editing?.index === index}
    <textarea
      value={editing.draft}
      aria-label={m.menu_edit_message()}
      oninput={event => {
        if (editing) {
          editing.draft = event.currentTarget.value
        }
      }}
      onkeydown={keys}
      onblur={() => (editing = null)}
      rows="3"
      {@attach focus}
      class={[
        "textarea textarea-sm w-full shrink-0 resize-none bg-base-100 text-sm",
        "leading-6 select-text",
      ]}
    ></textarea>
  {:else}
    <article
      use:contextmenu={() => actions(index)}
      class={[
        "group relative flex shrink-0 gap-3 py-2 pr-8",
        turn.role === "you" ? "bg-base-200 pl-4 hairline" : "pl-1",
      ]}
    >
      {#if turn.role === "you"}
        <span
          aria-hidden="true"
          class="absolute inset-y-0 left-0 w-1 bg-primary"
        ></span>
      {:else}
        <Logo plain class="mt-1 size-4 shrink-0 text-primary" />
      {/if}

      <div class="flex min-w-0 flex-1 flex-col gap-2">
        <p
          class={[
            "text-sm leading-6 whitespace-pre-wrap wrap-anywhere",
            "select-text",
          ]}
        >
          {#if turn.role === "you"}
            <span class="sr-only">{m.chat_you()}:</span>
          {/if}
          {turn.text}
        </p>

        {#if turn.move}
          {@render moved(turn.move)}
        {/if}
      </div>

      <button
        type="button"
        aria-label={m.chat_message_menu()}
        aria-haspopup="menu"
        onclick={event => menuBelow(event, actions(index))}
        class={[
          "btn btn-square btn-ghost btn-xs absolute top-1 right-1 opacity-0",
          "group-hover:opacity-100 focus-visible:opacity-100",
        ]}
      >
        <Icon icon="lucide:ellipsis" class="size-4" />
      </button>
    </article>

    {#if chat.branch?.at === index && chat.branch.threads.length > 1}
      {@const alts = chat.branch}

      <div
        class={[
          "flex shrink-0 items-center gap-1 self-end text-xs",
          "text-base-content/70",
        ]}
      >
        <button
          type="button"
          aria-label={m.chat_version_prev()}
          onclick={() => chat.pickThread(alts.pick - 1)}
          disabled={alts.pick === 0}
          class="btn btn-square btn-ghost btn-xs"
        >
          <Icon icon="lucide:chevron-left" class="size-4" />
        </button>

        <span class="tabular-nums">
          {alts.pick + 1}/{alts.threads.length}
        </span>

        <button
          type="button"
          aria-label={m.chat_version_next()}
          onclick={() => chat.pickThread(alts.pick + 1)}
          disabled={alts.pick === alts.threads.length - 1}
          class="btn btn-square btn-ghost btn-xs"
        >
          <Icon icon="lucide:chevron-right" class="size-4" />
        </button>
      </div>
    {/if}
  {/if}
{/each}

{#if chat.error}
  <p
    role="alert"
    class={[
      "flex shrink-0 items-start gap-2 text-sm text-error select-text",
      "wrap-anywhere",
    ]}
  >
    <Icon icon="lucide:circle-alert" class="mt-1 size-4 shrink-0" />
    <span class="min-w-0">{chat.error}</span>
  </p>
{/if}
