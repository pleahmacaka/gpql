<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { arrive, Icon, leave } from "@gpql/ui"

  type Props = {
    placeholder: string
    term: string
    index: number
    total: number
    onnext: () => void
    onprev: () => void
    onclose: () => void
  }

  let {
    placeholder,
    term = $bindable(),
    index,
    total,
    onnext,
    onprev,
    onclose,
  }: Props = $props()

  let box = $state<HTMLInputElement | null>(null)

  $effect(() => {
    box?.focus()
    box?.select()
  })

  function keys(event: KeyboardEvent) {
    if (event.isComposing) {
      return
    }

    if (event.key === "Enter") {
      event.preventDefault()
      event.shiftKey ? onprev() : onnext()
    }

    if (event.key === "Escape") {
      event.preventDefault()
      onclose()
    }
  }
</script>

<svelte:window
  onkeydown={event => {
    if (event.code === "KeyF" && event.ctrlKey) {
      event.preventDefault()
      box?.focus()
      box?.select()
    }
  }}
/>

<div
  role="search"
  in:arrive|global={{ from: "right", distance: 1 }}
  out:leave|global
  class={[
    "input input-sm w-64 max-w-full min-w-32 shrink gap-1 bg-base-100",
    "pr-1",
  ]}
>
  <Icon icon="lucide:search" class="size-4 shrink-0 text-base-content/60" />

  <input
    bind:this={box}
    bind:value={term}
    onkeydown={keys}
    {placeholder}
    aria-label={placeholder}
    spellcheck="false"
    class="min-w-0 grow select-text placeholder:text-base-content/60"
  />

  <span
    aria-live="polite"
    class={[
      "shrink-0 text-right text-xs whitespace-nowrap tabular-nums",
      total === 0 && term !== "" ? "text-error" : "text-base-content/70",
    ]}
  >
    {term === ""
      ? ""
      : total === 0
        ? m.find_none()
        : m.find_count({ index: index + 1, total })}
  </span>

  <button
    type="button"
    aria-label={m.find_previous()}
    onclick={onprev}
    disabled={total === 0}
    class="btn btn-square btn-ghost btn-xs"
  >
    <Icon icon="lucide:chevron-up" class="size-4" />
  </button>

  <button
    type="button"
    aria-label={m.find_next()}
    onclick={onnext}
    disabled={total === 0}
    class="btn btn-square btn-ghost btn-xs"
  >
    <Icon icon="lucide:chevron-down" class="size-4" />
  </button>

  <button
    type="button"
    aria-label={m.close()}
    onclick={onclose}
    class="btn btn-square btn-ghost btn-xs"
  >
    <Icon icon="lucide:x" class="size-4" />
  </button>
</div>
