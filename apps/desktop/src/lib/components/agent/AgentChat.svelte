<script lang="ts">
  import { Icon } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  let draft = $state("")

  async function send() {
    const text = draft

    draft = ""
    await workspace.say(text)
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault()
      void send()
    }
  }
</script>

<aside class="flex w-96 shrink-0 flex-col rounded-box bg-base-100 lift">
  <header class="flex items-center gap-2 px-4 pt-2 pb-1">
    <h2 class="flex-1 text-sm font-medium">{m.agent()}</h2>

    <button
      type="button"
      aria-label={m.chat_clear()}
      onclick={() => (workspace.chat = [])}
      class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
    >
      <Icon icon="lucide:eraser" class="size-4" />
    </button>

    <button
      type="button"
      aria-label={m.chat_close()}
      onclick={() => (workspace.chatOpen = false)}
      class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
    >
      <Icon icon="lucide:x" class="size-4" />
    </button>
  </header>

  <div class="min-h-0 flex-1 space-y-2 overflow-y-auto px-3 py-2">
    {#if workspace.chat.length === 0}
      <p class="px-1 py-6 text-center text-sm text-base-content/45">
        {m.chat_empty()}
      </p>
    {/if}

    {#each workspace.chat as turn, index (index)}
      <div
        class="rounded-field px-3 py-2 text-sm whitespace-pre-wrap {turn.role ===
        'you'
          ? 'bg-primary/10 text-primary'
          : 'bg-base-200'}"
      >
        {turn.text}
      </div>
    {/each}

    {#if workspace.chatBusy}
      <p class="flex items-center gap-2 px-1 text-xs text-base-content/45">
        <span class="loading loading-spinner loading-xs"></span>
        {m.chat_thinking()}
      </p>
    {/if}

    {#if workspace.chatError}
      <p class="px-1 text-xs text-error">{workspace.chatError}</p>
    {/if}
  </div>

  <div class="p-2">
    <textarea
      bind:value={draft}
      onkeydown={keys}
      placeholder={m.chat_placeholder()}
      rows="3"
      class="w-full resize-none rounded-field bg-base-200 px-3 py-2 text-sm
        outline-none placeholder:text-base-content/25"
    ></textarea>

    <button
      type="button"
      onclick={send}
      disabled={workspace.chatBusy}
      class="mt-1 w-full rounded-field bg-primary py-1.5 text-sm
        text-primary-content transition-colors hover:bg-primary/90
        disabled:opacity-50"
    >
      {m.chat_send()}
    </button>
  </div>
</aside>
