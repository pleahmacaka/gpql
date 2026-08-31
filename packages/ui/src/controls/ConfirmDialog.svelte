<script lang="ts">
  import { fade, scale } from "svelte/transition"

  import { Icon } from "../icons"
  import { pop, veil } from "../motion"

  type Props = {
    title: string
    body?: string
    confirm: string
    cancel: string
    icon?: string
    danger?: boolean
    onconfirm: () => void
    oncancel: () => void
  }

  let {
    title,
    body = "",
    confirm,
    cancel,
    icon = "lucide:trash-2",
    danger = true,
    onconfirm,
    oncancel,
  }: Props = $props()

  let keep = $state<HTMLButtonElement | null>(null)

  $effect(() => {
    keep?.focus()
  })

  function keys(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation()
      oncancel()
    }

    if (event.key === "Enter") {
      event.preventDefault()
      onconfirm()
    }
  }
</script>

<svelte:window onkeydown={keys} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  transition:fade={veil()}
  onclick={oncancel}
  class="fixed inset-0 z-70 grid place-items-center bg-base-300/45 p-6"
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    transition:scale={pop()}
    onclick={event => event.stopPropagation()}
    role="alertdialog"
    aria-label={title}
    tabindex="-1"
    class="flex w-full max-w-sm flex-col gap-4 rounded-box floating p-4 lift"
  >
    <div class="flex items-start gap-3">
      <span
        class="grid size-8 shrink-0 place-items-center rounded-field
          {danger ? 'bg-error/15 text-error' : 'bg-base-200 text-base-content'}"
      >
        <Icon {icon} class="size-4" />
      </span>

      <div class="min-w-0 flex-1">
        <h2 class="text-sm font-medium">{title}</h2>

        {#if body}
          <p class="pt-1 text-xs break-all text-base-content/45">{body}</p>
        {/if}
      </div>
    </div>

    <div class="flex items-center justify-end gap-2">
      <button
        bind:this={keep}
        type="button"
        onclick={oncancel}
        class="rounded-field bg-base-200 px-3 py-2 text-xs hover:bg-base-300"
      >
        {cancel}
      </button>

      <button
        type="button"
        onclick={onconfirm}
        class="rounded-field px-3 py-2 text-xs
          {danger
          ? 'bg-error text-error-content hover:bg-error/90'
          : 'bg-primary text-primary-content hover:bg-primary/90'}"
      >
        {confirm}
      </button>
    </div>
  </div>
</div>
