<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { fade, scale } from "svelte/transition"

  import { Icon, Keycap, pop, veil } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  let prompt = $state("")
  let provider = $state("")
  let box = $state<HTMLTextAreaElement | null>(null)

  let choices = $derived(
    workspace.providers.map(entry => ({ id: entry.id, name: entry.name })),
  )

  let chosen = $derived(provider || (choices[0]?.id ?? ""))
  let base = $derived(workspace.query.sql.trim())

  $effect(() => {
    box?.focus()
  })

  async function ask() {
    if (prompt.trim() === "") {
      return
    }

    await workspace.ask(chosen, prompt)
    onclose()
  }
</script>

<svelte:window
  onkeydown={event => event.key === "Escape" && onclose()}
/>

<div
  transition:fade={veil()}
  class="fixed inset-0 z-50 scrim"
  role="presentation"
  onclick={onclose}
></div>

<div
  transition:scale={pop()}
  class="fixed inset-x-0 top-24 z-50 mx-auto flex h-fit w-xl max-w-11/12
    flex-col overflow-hidden rounded-box floating lift"
  role="dialog"
  aria-label={m.ai_write_title()}
>
  <header class="flex items-center gap-2 px-5 pt-4 pb-2">
    <Icon icon="lucide:sparkles" class="size-4 text-accent" />

    <h2 class="flex-1 text-sm font-medium">{m.ai_write_title()}</h2>

    <button
      type="button"
      aria-label={m.close()}
      onclick={onclose}
      class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
    >
      <Icon icon="lucide:x" class="size-4" />
    </button>
  </header>

  <p class="px-5 pb-3 text-xs text-base-content/45">
    {base === "" ? m.ai_write_fresh() : m.ai_write_base()}
  </p>

  <div class="px-5">
    <textarea
      bind:this={box}
      bind:value={prompt}
      onkeydown={event => {
        if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
          event.preventDefault()
          ask()
        }
      }}
      placeholder={m.ai_write_hint()}
      class="h-24 w-full resize-none rounded-field bg-base-200 px-3 py-2 text-sm
        outline-none select-text placeholder:text-base-content/30"
    ></textarea>
  </div>

  {#if base !== ""}
    <pre
      class="mx-5 mt-2 max-h-24 overflow-auto rounded-field bg-base-200 px-3
        py-2 font-mono text-xs text-base-content/55">{base}</pre>
  {/if}

  <footer class="flex items-center gap-2 px-5 py-4">
    {#if choices.length > 1}
      <select
        bind:value={provider}
        class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs
          text-base-content/60 outline-none"
      >
        {#each choices as entry (entry.id)}
          <option value={entry.id}>{entry.name}</option>
        {/each}
      </select>
    {/if}

    <Keycap keys={["ctrl", "enter"]} />

    <span class="flex-1"></span>

    <button
      type="button"
      onclick={onclose}
      class="rounded-field px-3 py-2 text-sm text-base-content/60
        hover:bg-base-200"
    >
      {m.cancel()}
    </button>

    <button
      type="button"
      onclick={ask}
      disabled={workspace.busy || prompt.trim() === ""}
      class="flex items-center gap-2 rounded-field bg-primary px-3 py-2
        text-sm text-primary-content hover:bg-primary/90
        disabled:bg-base-200 disabled:text-base-content/30"
    >
      {#if workspace.busy}
        <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
      {/if}

      {m.ask_write()}
    </button>
  </footer>
</div>
