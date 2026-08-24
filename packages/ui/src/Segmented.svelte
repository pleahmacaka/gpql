<script lang="ts">
  type Props = {
    options: { value: string; label: string }[]
    value: string
    onpick?: (value: string) => void
  }

  let { options, value = $bindable(), onpick }: Props = $props()
</script>

<div class="flex gap-1 rounded-selector bg-base-200 p-1">
  {#each options as option (option.value)}
    <button
      type="button"
      onclick={() => {
        value = option.value
        onpick?.(option.value)
      }}
      aria-pressed={value === option.value}
      class="flex-1 rounded-selector py-1.5 text-sm transition-colors
        {value === option.value
        ? 'bg-base-100 font-medium hairline'
        : 'text-base-content/55 hover:text-base-content'}"
    >
      {option.label}
    </button>
  {/each}
</div>
