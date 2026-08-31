<script lang="ts">
  import { fade } from "svelte/transition"

  import { Icon, Logo, drag, menu, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  import ChatMessages from "./ChatMessages.svelte"

  let chat = $derived(workspace.chat)

  let draft = $state("")
  let box = $state<HTMLInputElement | null>(null)
  let log = $state<HTMLDivElement | null>(null)
  let orbSize = $state({ width: 320, height: 208 })
  let showLog = $state(true)

  $effect(() => {
    if (chat.dock === "orb") {
      box?.focus()
    }
  })

  $effect(() => {
    chat.turns.length
    chat.busy

    const element = log

    if (element) {
      requestAnimationFrame(() => {
        element.scrollTop = element.scrollHeight
      })
    }
  })

  async function send() {
    const text = draft

    draft = ""
    await chat.send(text)
  }

  function history(event: MouseEvent) {
    menu.show(
      event,
      chat.saved.map(entry => ({
        label: entry.title || m.chat_untitled(),
        icon: "lucide:message-circle",
        run: () => chat.open(entry.id),
      })),
    )
  }

  function resizeOrb(event: PointerEvent) {
    const startX = event.clientX
    const startY = event.clientY
    const start = { ...orbSize }

    drag(event, moved => {
      orbSize = {
        width: Math.min(Math.max(start.width + startX - moved.clientX, 240), 640),
        height: Math.min(
          Math.max(start.height + startY - moved.clientY, 120),
          480,
        ),
      }
    })
  }
</script>

{#snippet newChat()}
  <button
    type="button"
    aria-label={m.chat_new()}
    onclick={() => chat.start()}
    class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
  >
    <Icon icon="lucide:plus" class="size-4" />
  </button>
{/snippet}

{#if chat.dock === "panel"}
  <aside class="flex w-80 shrink-0 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-center gap-1 px-4 pt-2 pb-1">
      <h2 class="min-w-0 flex-1 truncate text-sm font-medium">
        {chat.title || m.agent()}
      </h2>

      <button
        type="button"
        aria-label={m.chat_history()}
        onclick={history}
        class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
      >
        <Icon icon="lucide:history" class="size-4" />
      </button>

      {@render newChat()}

      <button
        type="button"
        aria-label={m.close()}
        onclick={() => (chat.dock = "off")}
        class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
      >
        <Icon icon="lucide:x" class="size-4" />
      </button>
    </header>

    <div
      bind:this={log}
      class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-3 py-2"
    >
      {#if chat.turns.length === 0}
        <p class="px-1 py-6 text-center text-sm text-base-content/45">
          {m.chat_empty()}
        </p>
      {/if}

      <ChatMessages />

      {#if chat.busy}
        <p class="flex items-center gap-2 px-1 text-xs text-base-content/45">
          <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
          {m.chat_thinking()}
        </p>
      {/if}
    </div>

    <div class="p-2">
      <textarea
        bind:value={draft}
        onkeydown={event => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault()
            void send()
          }
        }}
        placeholder={m.chat_placeholder()}
        rows="2"
        class="w-full resize-none rounded-field bg-base-200 px-3 py-2 text-xs
          outline-none placeholder:text-base-content/25"
      ></textarea>

      <button
        type="button"
        onclick={send}
        disabled={chat.busy || draft.trim() === ""}
        class="mt-1 w-full rounded-field bg-primary py-2 text-xs
          text-primary-content disabled:bg-base-200
          disabled:text-base-content/30"
      >
        {m.chat_send()}
      </button>
    </div>
  </aside>
{/if}

{#if chat.dock === "orb"}
  <div
    transition:fade={veil()}
    class="fixed bottom-6 z-40 flex max-w-11/12 flex-col gap-2
      {chat.side === 'center'
      ? 'inset-x-0 mx-auto items-center'
      : chat.side === 'right'
        ? 'right-6 items-end'
        : 'left-6 items-start'}"
  >
    {#if showLog && (chat.turns.length > 0 || chat.error)}
      <div class="relative" style:width="{orbSize.width}px">
        <button
          type="button"
          aria-label="Resize chat"
          onpointerdown={resizeOrb}
          class="absolute -top-1 -left-1 z-10 size-3 cursor-nwse-resize
            rounded-full bg-base-content/15 hover:bg-primary/60"
        ></button>

        <button
          type="button"
          aria-label={m.close()}
          onclick={() => (chat.dock = "off")}
          class="absolute -top-2 -right-2 z-10 grid size-6 place-items-center
            rounded-full floating text-base-content/45 lift
            hover:text-base-content"
        >
          <Icon icon="lucide:x" class="size-4" />
        </button>

        <div
          bind:this={log}
          style:max-height="{orbSize.height}px"
          class="flex w-full flex-col gap-2 overflow-y-auto rounded-box
            floating p-3 lift"
        >
          <ChatMessages />
        </div>
      </div>
    {/if}

    {#snippet mark()}
      <button
        type="button"
        aria-label={m.chat_history()}
        onclick={() => (showLog = !showLog)}
        oncontextmenu={history}
        class="grid size-9 shrink-0 place-items-center rounded-full
          bg-primary lift"
      >
        <Logo class="size-4 text-primary-content" />
      </button>
    {/snippet}

    <div class="flex items-center gap-3" style:width="{orbSize.width}px">
      {#if chat.side !== "right"}
        {@render mark()}
      {/if}

      <div
        class="flex min-w-0 flex-1 items-center gap-2 rounded-full floating
          py-2 pr-2 pl-4 lift"
      >
        <input
          bind:this={box}
          bind:value={draft}
          onkeydown={event => {
            if (event.key === "Enter") {
              event.preventDefault()
              void send()
            }
          }}
          placeholder={m.orb_hint()}
          disabled={chat.busy}
          class="min-w-0 flex-1 bg-transparent text-xs outline-none
            select-text placeholder:text-base-content/35"
        />

        {#if chat.busy}
          <Icon
            icon="lucide:loader-circle"
            class="size-4 animate-spin text-base-content/40"
          />
        {/if}

        {@render newChat()}

        <button
          type="button"
          aria-label={m.orb_pin()}
          onclick={() => (chat.dock = "panel")}
          class="rounded-full p-2 text-base-content/40 hover:bg-base-200
            hover:text-base-content"
        >
          <Icon icon="lucide:panel-right" class="size-4" />
        </button>
      </div>

      {#if chat.side === "right"}
        {@render mark()}
      {/if}
    </div>
  </div>
{/if}
