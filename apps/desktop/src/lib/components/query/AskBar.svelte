<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  let prompt = $state("")
  let provider = $state("")

  let choices = $derived([
    ...(workspace.agent ? [{ id: "acp", name: "Agent" }] : []),
    ...workspace.providers.map(entry => ({ id: entry.id, name: entry.name })),
  ])

  let chosen = $derived(provider || (choices[0]?.id ?? ""))

  async function ask() {
    await workspace.ask(chosen, prompt)
    prompt = ""
  }
</script>

{#if choices.length > 0}
  <div class="mx-3 mb-2 flex items-center gap-2 rounded-field bg-base-200 px-3 py-1.5">
    <Icon icon="lucide:sparkles" class="size-3.5 shrink-0 text-accent" />

    <input
      bind:value={prompt}
      onkeydown={event => event.key === "Enter" && ask()}
      placeholder={m.ask_placeholder()}
      class="min-w-0 flex-1 bg-transparent text-sm outline-none
        placeholder:text-base-content/30"
    />

    {#if choices.length > 1}
      <select
        bind:value={provider}
        class="cursor-pointer bg-transparent text-xs text-base-content/50 outline-none"
      >
        {#each choices as entry (entry.id)}
          <option value={entry.id}>{entry.name}</option>
        {/each}
      </select>
    {/if}

    <button
      type="button"
      onclick={ask}
      disabled={workspace.busy || prompt.trim() === ""}
      class="rounded-field px-2 py-1 text-xs text-primary hover:bg-base-300
        disabled:text-base-content/25"
    >
      {m.ask_write()}
    </button>
  </div>
{/if}
