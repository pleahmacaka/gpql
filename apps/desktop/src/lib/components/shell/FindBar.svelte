<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon } from "@gpql/ui"

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

<div
  class="flex items-center gap-1 rounded-field bg-base-200 px-2 py-1 hairline"
>
  <Icon icon="lucide:search" class="size-3.5 shrink-0 text-base-content/35" />

  <input
    bind:this={box}
    bind:value={term}
    onkeydown={keys}
    {placeholder}
    class="w-40 bg-transparent text-xs outline-none select-text
      placeholder:text-base-content/30"
  />

  <span class="w-12 text-right text-xs text-base-content/40">
    {term === ""
      ? ""
      : total === 0
        ? m.find_none()
        : m.find_count({ index: index + 1, total })}
  </span>

  <button
    type="button"
    aria-label="Previous match"
    onclick={onprev}
    disabled={total === 0}
    class="rounded-selector p-0.5 text-base-content/45 hover:text-base-content
      disabled:text-base-content/20"
  >
    <Icon icon="lucide:chevron-down" class="size-3.5 rotate-180" />
  </button>

  <button
    type="button"
    aria-label="Next match"
    onclick={onnext}
    disabled={total === 0}
    class="rounded-selector p-0.5 text-base-content/45 hover:text-base-content
      disabled:text-base-content/20"
  >
    <Icon icon="lucide:chevron-down" class="size-3.5" />
  </button>

  <button
    type="button"
    aria-label={m.close()}
    onclick={onclose}
    class="rounded-selector p-0.5 text-base-content/40 hover:text-base-content"
  >
    <Icon icon="lucide:x" class="size-3.5" />
  </button>
</div>
