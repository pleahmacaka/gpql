<script lang="ts" module>
  let draft = $state("")
  let size = $state({ width: 22, height: 14 })
</script>

<script lang="ts">
  import { fade } from "svelte/transition"

  import {
    arrive,
    depart,
    drag,
    EmptyState,
    Icon,
    Logo,
    Panel,
    rem,
    veil,
  } from "@gpql/ui"

  import { menuBelow } from "$lib/components/query/anchor"
  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  import ChatMessages from "./ChatMessages.svelte"

  let chat = $derived(workspace.chat)

  let box = $state<HTMLInputElement | null>(null)
  let log = $state<HTMLDivElement | null>(null)
  let showLog = $state(true)
  let stuck = $state(true)

  let talking = $derived(
    chat.turns.length > 0 || chat.busy || chat.error !== "",
  )

  $effect(() => {
    if (chat.dock === "orb") {
      box?.focus()
    }
  })

  $effect(() => {
    void chat.turns.length
    void chat.busy
    void chat.error

    const element = log

    if (element && stuck) {
      requestAnimationFrame(() => {
        element.scrollTop = element.scrollHeight
      })
    }
  })

  function follow() {
    if (log) {
      stuck = log.scrollHeight - log.scrollTop - log.clientHeight < rem(4)
    }
  }

  async function send() {
    const text = draft

    if (chat.busy || text.trim() === "") {
      return
    }

    draft = ""
    stuck = true
    showLog = true
    await chat.send(text)
  }

  function enter(event: KeyboardEvent) {
    if (event.key !== "Enter" || event.shiftKey || event.isComposing) {
      return
    }

    event.preventDefault()
    void send()
  }

  function history(event: MouseEvent) {
    menuBelow(
      event,
      chat.saved.map(entry => ({
        label: entry.title || m.chat_untitled(),
        icon: "lucide:message-square",
        run: () => chat.open(entry.id),
      })),
    )
  }

  function resize(event: PointerEvent) {
    const from = { x: event.clientX, y: event.clientY }
    const start = { ...size }
    const unit = rem(1)
    const flip = chat.side === "left" ? -1 : 1

    drag(event, moved => {
      size = {
        width: Math.min(
          Math.max(start.width + (flip * (from.x - moved.clientX)) / unit, 16),
          40,
        ),
        height: Math.min(
          Math.max(start.height + (from.y - moved.clientY) / unit, 8),
          30,
        ),
      }
    })
  }
</script>

{#snippet tools()}
  <button
    type="button"
    aria-label={m.chat_history()}
    aria-haspopup="menu"
    disabled={chat.saved.length === 0}
    onclick={history}
    class="btn btn-square btn-ghost btn-sm"
  >
    <Icon icon="lucide:history" class="size-4" />
  </button>

  <button
    type="button"
    aria-label={m.chat_new()}
    disabled={chat.turns.length === 0}
    onclick={() => chat.start()}
    class="btn btn-square btn-ghost btn-sm"
  >
    <Icon icon="lucide:square-pen" class="size-4" />
  </button>
{/snippet}

{#snippet thinking()}
  {#if chat.busy}
    <p
      in:fade={veil()}
      role="status"
      class="flex h-8 shrink-0 items-center gap-2 text-sm text-base-content/70"
    >
      <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
      {m.chat_thinking()}
    </p>
  {/if}
{/snippet}

{#if chat.dock === "panel"}
  <div
    in:arrive|global={{ from: "right", distance: 1 }}
    out:depart|global={{ to: "right", distance: 1 }}
    class="flex min-h-0 w-80 shrink-0"
  >
    <Panel glass={false} label={m.agent()} class="min-w-0 flex-1">
      <header
        class={[
          "flex h-11 shrink-0 items-center gap-1 border-b",
          "border-base-content/10 pr-2 pl-3",
        ]}
      >
        <Logo plain class="size-4 shrink-0 text-primary" />

        <h2 class="min-w-0 flex-1 truncate pl-1 text-sm font-medium">
          {chat.title || m.agent()}
        </h2>

        {@render tools()}

        <button
          type="button"
          aria-label={m.chat_close()}
          onclick={() => (chat.dock = "off")}
          class="btn btn-square btn-ghost btn-sm"
        >
          <Icon icon="lucide:x" class="size-4" />
        </button>
      </header>

      <div
        bind:this={log}
        onscroll={follow}
        role="log"
        aria-label={m.chat_log()}
        class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3"
      >
        {#if !talking}
          <EmptyState art="mark" title={m.chat_empty()} class="flex-1" />
        {/if}

        <ChatMessages />

        {@render thinking()}
      </div>

      <footer class="shrink-0 border-t border-base-content/10 p-2">
        <div
          class={[
            "flex items-end gap-2 bg-base-100 p-1 hairline",
            "focus-within:outline-2 focus-within:outline-primary",
          ]}
        >
          <textarea
            bind:value={draft}
            onkeydown={enter}
            aria-label={m.chat_message()}
            placeholder={m.chat_placeholder()}
            rows="2"
            class={[
              "min-w-0 flex-1 resize-none bg-transparent px-2 py-1 text-sm",
              "leading-6 outline-none select-text",
              "placeholder:text-base-content/60",
            ]}
          ></textarea>

          <button
            type="button"
            aria-label={m.chat_send()}
            onclick={send}
            disabled={chat.busy || draft.trim() === ""}
            class="btn btn-square btn-primary btn-sm"
          >
            <Icon icon="lucide:arrow-up" class="size-4" />
          </button>
        </div>
      </footer>
    </Panel>
  </div>
{/if}

{#if chat.dock === "orb"}
  <div
    in:arrive|global={{ from: "down", distance: 1 }}
    out:depart|global={{ to: "down", distance: 1 }}
    class={[
      "fixed bottom-6 z-40 flex max-w-11/12 flex-col gap-2",
      chat.side === "center" && "inset-x-0 mx-auto items-center",
      chat.side === "right" && "right-6 items-end",
      chat.side === "left" && "left-6 items-start",
    ]}
    style:width="{size.width}rem"
  >
    {#if showLog && talking}
      <div
        transition:fade={veil()}
        class="floating lift relative flex w-full flex-col"
        style:max-height="{size.height}rem"
      >
        <button
          type="button"
          aria-label={m.resize()}
          onpointerdown={resize}
          class={[
            "absolute -top-1 z-10 size-3 bg-base-content/40 transition-colors",
            "hover:bg-primary",
            chat.side === "left"
              ? "-right-1 cursor-nesw-resize"
              : "-left-1 cursor-nwse-resize",
          ]}
        ></button>

        <div
          bind:this={log}
          onscroll={follow}
          role="log"
          aria-label={m.chat_log()}
          class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3"
        >
          <ChatMessages />

          {@render thinking()}
        </div>
      </div>
    {/if}

    <div
      class={[
        "flex w-full items-center gap-2",
        chat.side === "right" && "flex-row-reverse",
      ]}
    >
      <button
        type="button"
        aria-label={showLog ? m.chat_hide_log() : m.chat_show_log()}
        aria-expanded={showLog && talking}
        onclick={() => (talking ? (showLog = !showLog) : box?.focus())}
        class="btn btn-square btn-primary shrink-0 lift"
      >
        <Logo plain class="size-5" />
      </button>

      <div
        class={[
          "floating lift relative flex min-w-0 flex-1 items-center gap-1 py-1",
          "pr-1 pl-3 focus-within:outline-2 focus-within:outline-primary",
        ]}
      >
        <span
          aria-hidden="true"
          class="hud hud-lit hud-small pointer-events-none absolute inset-0"
        ></span>

        <input
          bind:this={box}
          bind:value={draft}
          onkeydown={enter}
          placeholder={m.orb_hint()}
          aria-label={m.orb_hint()}
          class={[
            "min-w-0 flex-1 bg-transparent py-1 text-sm outline-none",
            "select-text placeholder:text-base-content/60",
          ]}
        />

        {#if chat.busy}
          <Icon
            icon="lucide:loader-circle"
            class="size-4 shrink-0 animate-spin text-base-content/60"
          />
        {/if}

        {@render tools()}

        <button
          type="button"
          aria-label={m.orb_pin()}
          onclick={() => (chat.dock = "panel")}
          class="btn btn-square btn-ghost btn-sm"
        >
          <Icon icon="lucide:panel-right" class="size-4" />
        </button>
      </div>
    </div>
  </div>
{/if}
