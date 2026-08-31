<script lang="ts">
  type Props = {
    options: { value: string; label: string }[]
    value: string
    onpick?: (value: string) => void
  }

  let { options, value = $bindable(), onpick }: Props = $props()

  let strip = $state<HTMLDivElement | null>(null)
  let pill = $state({ left: 0, width: 0 })
  let settled = $state(false)

  function measure(active: string) {
    const found = strip?.querySelector<HTMLElement>(
      `[data-option="${CSS.escape(active)}"]`,
    )

    if (!found) {
      return
    }

    pill = { left: found.offsetLeft, width: found.offsetWidth }
  }

  $effect(() => {
    const active = value

    if (!strip) {
      return
    }

    measure(active)

    if (settled) {
      return
    }

    const frame = requestAnimationFrame(() => {
      measure(active)
      settled = true
    })

    return () => cancelAnimationFrame(frame)
  })

  $effect(() => {
    if (!strip) {
      return
    }

    const watcher = new ResizeObserver(() => measure(value))

    watcher.observe(strip)

    return () => watcher.disconnect()
  })
</script>

<div
  bind:this={strip}
  class="relative flex gap-1 rounded-selector bg-base-200 p-1"
>
  <span
    aria-hidden="true"
    class="absolute top-1 bottom-1 left-0 rounded-selector bg-base-100 hairline transition-all duration-200 ease-out {settled ? '' : 'duration-0'}"
    style:transform="translateX({pill.left}px)"
    style:width="{pill.width}px"
  ></span>

  {#each options as option (option.value)}
    <button
      type="button"
      data-option={option.value}
      onclick={() => {
        value = option.value
        onpick?.(option.value)
      }}
      aria-pressed={value === option.value}
      class="relative z-10 flex-1 rounded-selector py-2 text-sm
        transition-colors {value === option.value
        ? 'font-medium'
        : 'text-base-content/55 hover:text-base-content'}"
    >
      {option.label}
    </button>
  {/each}
</div>
