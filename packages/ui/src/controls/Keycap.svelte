<script lang="ts">
  type Props = { keys: string[]; class?: string }

  let { keys, class: extra = "" }: Props = $props()

  const shown: Record<string, string> = {
    ctrl: "Ctrl",
    shift: "Shift",
    alt: "Alt",
    enter: "↵",
    escape: "Esc",
    space: "Space",
  }
</script>

<span class={["flex items-center gap-1", extra]}>
  {#each keys as key, index (index)}
    {#if index > 0}
      <span aria-hidden="true" class="text-xs text-base-content/40">+</span>
    {/if}

    <kbd
      class={[
        "min-w-5 bg-base-200 px-1 py-1 text-center text-xs leading-none",
        "text-base-content/70 tabular-nums hairline",
      ]}
    >
      {shown[key] ?? key.toUpperCase()}
    </kbd>
  {/each}
</span>
