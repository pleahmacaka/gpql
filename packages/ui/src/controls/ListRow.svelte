<script lang="ts">
  import { Icon } from "../icons"

  type Props = {
    icon: string
    title: string
    detail?: string
    trailing?: string
    tone?: "plain" | "bad"
    shaking?: boolean
    busy?: boolean
    onclick: () => void
    onedit?: () => void
    ondismiss?: () => void
  }

  let {
    icon,
    title,
    detail = "",
    trailing = "lucide:arrow-right",
    tone = "plain",
    shaking = false,
    busy = false,
    onclick,
    onedit,
    ondismiss,
  }: Props = $props()
</script>

<div
  class="group flex min-w-0 items-center rounded-field hover:bg-base-200
    {shaking ? 'animate-shake' : ''}"
>
  <button
    type="button"
    {onclick}
    data-blocked={tone === "bad"}
    class="flex min-w-0 flex-1 items-center gap-3 py-2 pl-3 text-left"
  >
    <Icon
      {icon}
      class="size-4 shrink-0 stroke-current stroke-1 {tone === 'bad'
        ? 'text-error'
        : 'text-base-content/40'}"
    />

    <span class="min-w-0 flex-1">
      <span class="block truncate text-sm {tone === 'bad' ? 'text-error' : ''}">
        {title}
      </span>

      {#if detail}
        <span
          class="block truncate text-xs {tone === 'bad'
            ? 'text-error'
            : 'text-base-content/45'}"
        >
          {detail}
        </span>
      {/if}
    </span>
  </button>

  <span class="flex w-20 shrink-0 items-center justify-end gap-0.5 pr-3">
    {#if onedit}
      <button
        type="button"
        aria-label="Edit {title}"
        onclick={onedit}
        class="grid size-5 place-items-center rounded-selector
          text-base-content/30 opacity-0 group-hover:opacity-100
          hover:text-primary focus-visible:opacity-100"
      >
        <Icon icon="lucide:pencil" class="size-3.5" />
      </button>
    {/if}

    {#if ondismiss}
      <button
        type="button"
        aria-label="Forget {title}"
        onclick={ondismiss}
        class="grid size-5 place-items-center rounded-selector
          text-base-content/30 opacity-0 group-hover:opacity-100
          hover:text-error focus-visible:opacity-100"
      >
        <Icon icon="lucide:x" class="size-3.5" />
      </button>
    {/if}

    {#if busy}
      <Icon
        icon="lucide:loader-circle"
        class="size-4 animate-spin text-base-content/50"
      />
    {:else}
      <Icon
        icon={trailing}
        class="size-4 text-base-content/30 group-hover:text-base-content/60"
      />
    {/if}
  </span>
</div>
