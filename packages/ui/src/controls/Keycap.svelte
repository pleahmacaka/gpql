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

<span class="flex items-center gap-0.5 {extra}">
  {#each keys as key, index (key)}
    {#if index > 0}
      <span class="text-xs text-base-content/25">+</span>
    {/if}

    <kbd
      class="rounded-selector bg-base-200 px-1 py-0.5 font-mono text-xs
        leading-none text-base-content/55 hairline"
    >
      {shown[key] ?? key.toUpperCase()}
    </kbd>
  {/each}
</span>
