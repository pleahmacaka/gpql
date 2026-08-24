<script lang="ts">
  import { Icon } from "./icons"

  type Props = {
    icon: string
    title: string
    detail?: string
    trailing?: string
    onclick: () => void
    ondismiss?: () => void
  }

  let {
    icon,
    title,
    detail = "",
    trailing = "lucide:arrow-right",
    onclick,
    ondismiss,
  }: Props = $props()
</script>

<div class="group flex min-w-0 items-center rounded-field hover:bg-base-200">
  <button
    type="button"
    {onclick}
    class="flex min-w-0 flex-1 items-center gap-3 px-3 py-2 text-left"
  >
    <Icon {icon} class="size-4 shrink-0 text-base-content/40" />

    <span class="min-w-0 flex-1">
      <span class="block truncate text-sm">{title}</span>

      {#if detail}
        <span class="block truncate text-xs text-base-content/45">
          {detail}
        </span>
      {/if}
    </span>

    <Icon
      icon={trailing}
      class="size-4 shrink-0 text-base-content/30 group-hover:text-base-content/60"
    />
  </button>

  {#if ondismiss}
    <button
      type="button"
      aria-label="Forget {title}"
      onclick={ondismiss}
      class="mr-2 rounded-selector p-1 text-base-content/30 opacity-0
        group-hover:opacity-100 hover:text-error focus-visible:opacity-100"
    >
      <Icon icon="lucide:x" class="size-3.5" />
    </button>
  {/if}
</div>
