<script lang="ts">
  import { fade } from "svelte/transition"

  import { Dialog, Dropdown, type Ending, Icon, Keycap, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  import Code from "./Code.svelte"

  type Props = { onclose: () => void }

  let { onclose }: Props = $props()

  let prompt = $state("")
  let provider = $state("")
  let writing = $state(false)
  let failure = $state("")
  let ending = $state<Ending>("cancel")

  let choices = $derived(
    workspace.providers.map(entry => ({ value: entry.id, label: entry.name })),
  )

  let chosen = $derived(provider || (choices[0]?.value ?? ""))
  let base = $derived(workspace.query.sql.trim())

  async function ask() {
    if (prompt.trim() === "" || writing || workspace.query.busy) {
      return
    }

    writing = true
    failure = ""

    try {
      await workspace.ask(chosen, prompt)
    } finally {
      writing = false
    }

    if (workspace.query.composeError) {
      failure = workspace.query.composeError

      return
    }

    ending = "confirm"
    onclose()
  }

  function keys(event: KeyboardEvent) {
    if (
      event.key === "Enter" &&
      (event.ctrlKey || event.metaKey) &&
      !event.isComposing
    ) {
      event.preventDefault()
      void ask()
    }
  }
</script>

<Dialog
  label={m.ai_write_title()}
  {onclose}
  dismiss={m.cancel()}
  size="md"
  place="top"
  {ending}
>
  <header
    class={[
      "flex items-center gap-3 border-b border-base-content/10 py-3 pr-3",
      "pl-6",
    ]}
  >
    <Icon icon="lucide:sparkles" class="size-4 shrink-0 text-primary" />

    <div class="min-w-0 flex-1">
      <h2 class="text-base font-semibold tracking-tight">
        {m.ai_write_title()}
      </h2>
      <p class="text-xs text-base-content/70">
        {base === "" ? m.ai_write_fresh() : m.ai_write_base()}
      </p>
    </div>

    <button
      type="button"
      aria-label={m.close()}
      onclick={onclose}
      class="btn btn-square btn-ghost btn-sm"
    >
      <Icon icon="lucide:x" class="size-4" />
    </button>
  </header>

  <div class="flex min-h-0 flex-col gap-4 overflow-y-auto px-6 py-4">
    <label class="flex flex-col gap-2">
      <span class="text-xs text-base-content/70">{m.ask_prompt()}</span>

      <textarea
        data-autofocus
        bind:value={prompt}
        onkeydown={keys}
        readonly={writing}
        placeholder={m.ai_write_hint()}
        class={[
          "textarea textarea-sm h-28 w-full resize-none bg-base-100 text-sm",
          "leading-6 select-text placeholder:text-base-content/60",
        ]}
      ></textarea>
    </label>

    {#if base !== ""}
      <section class="flex flex-col gap-2">
        <span class="text-xs text-base-content/70">{m.ask_base()}</span>

        <div class="max-h-28 overflow-y-auto bg-base-200 px-3 py-2 hairline">
          <Code code={base} class="text-xs leading-5" />
        </div>
      </section>
    {/if}

    <div class="min-h-5">
      {#if writing}
        <p
          in:fade={veil()}
          role="status"
          class="flex items-center gap-2 text-xs text-base-content/70"
        >
          <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
          {m.ask_writing()}
        </p>
      {:else if failure}
        <p
          in:fade={veil()}
          role="alert"
          class="flex items-start gap-2 text-xs text-error select-text"
        >
          <Icon icon="lucide:circle-alert" class="size-4 shrink-0" />
          <span class="min-w-0 wrap-anywhere">{failure}</span>
        </p>
      {/if}
    </div>
  </div>

  <footer
    class="flex items-center gap-2 border-t border-base-content/10 px-6 py-4"
  >
    {#if choices.length > 1}
      <Dropdown
        small
        label={m.ask_model()}
        options={choices}
        value={chosen}
        onpick={next => (provider = next)}
      />
    {/if}

    <span class="flex-1"></span>

    <button
      type="button"
      onclick={onclose}
      class="btn btn-ghost btn-sm font-medium"
    >
      {m.cancel()}
    </button>

    <button
      type="button"
      onclick={ask}
      disabled={writing || workspace.query.busy || prompt.trim() === ""}
      aria-keyshortcuts="Control+Enter"
      class="btn btn-primary btn-sm font-medium"
    >
      {m.ask_write()}
      <Keycap keys={["ctrl", "enter"]} />
    </button>
  </footer>
</Dialog>
