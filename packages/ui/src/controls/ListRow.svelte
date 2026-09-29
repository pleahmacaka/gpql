<script lang="ts">
  import { Icon } from "../icons"

  type Props = {
    icon: string
    title: string
    detail?: string
    trailing?: string | null
    tone?: "plain" | "bad"
    active?: boolean
    shaking?: boolean
    busy?: boolean
    editLabel?: string
    dismissLabel?: string
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
    active = false,
    shaking = false,
    busy = false,
    editLabel = `Edit ${title}`,
    dismissLabel = `Forget ${title}`,
    onclick,
    onedit,
    ondismiss,
  }: Props = $props()

  let bad = $derived(tone === "bad")
</script>

<div
  class={[
    "group relative flex min-w-0 items-center transition-colors",
    active ? "bg-primary/10" : "hover:bg-base-content/5",
    bad && "bg-error/5",
    shaking && "animate-shake",
  ]}
>
  {#if active}
    <span
      aria-hidden="true"
      class="absolute inset-y-0 left-0 w-1 bg-primary"
    ></span>
  {/if}

  <button
    type="button"
    {onclick}
    aria-current={active ? "true" : undefined}
    data-blocked={bad}
    class={[
      "flex min-w-0 flex-1 cursor-pointer items-center gap-3 py-2 pl-4",
      "text-left outline-offset-0",
      bad && "cursor-not-allowed",
    ]}
  >
    <Icon
      {icon}
      class={[
        "size-4 shrink-0",
        bad ? "text-error" : active ? "text-primary" : "text-base-content/70",
      ]}
    />

    <span class="min-w-0 flex-1">
      <span
        class={[
          "block truncate text-sm",
          bad && "text-error",
          active && "font-medium",
        ]}
      >
        {title}
      </span>

      {#if detail}
        <span
          class={[
            "block truncate text-xs",
            bad ? "text-error" : "text-base-content/70",
          ]}
        >
          {detail}
        </span>
      {/if}
    </span>
  </button>

  <span class="flex shrink-0 items-center justify-end gap-1 pr-2">
    {#if onedit}
      <button
        type="button"
        aria-label={editLabel}
        onclick={onedit}
        class={[
          "grid size-6 cursor-pointer place-items-center text-base-content/70",
          "opacity-0 group-hover:opacity-100 hover:text-primary",
          "focus-visible:opacity-100",
        ]}
      >
        <Icon icon="lucide:pencil" class="size-4" />
      </button>
    {/if}

    {#if ondismiss}
      <button
        type="button"
        aria-label={dismissLabel}
        onclick={ondismiss}
        class={[
          "grid size-6 cursor-pointer place-items-center text-base-content/70",
          "opacity-0 group-hover:opacity-100 hover:text-error",
          "focus-visible:opacity-100",
        ]}
      >
        <Icon icon="lucide:x" class="size-4" />
      </button>
    {/if}

    {#if busy}
      <Icon
        icon="lucide:loader-circle"
        class="size-4 animate-spin text-base-content/70"
      />
    {:else if trailing}
      <Icon
        icon={trailing}
        class={[
          "size-4 text-base-content/40 transition-colors",
          "group-hover:text-base-content/80",
        ]}
      />
    {/if}
  </span>
</div>
