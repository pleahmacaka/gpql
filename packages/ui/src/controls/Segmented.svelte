<script lang="ts">
  import { Icon } from "../icons"
  import { rem } from "./rem"

  type Option = { value: string; label: string; icon?: string }

  type Props = {
    options: Option[]
    value: string
    onpick?: (value: string) => void
    label?: string
    small?: boolean
  }

  let {
    options,
    value = $bindable(),
    onpick,
    label,
    small = false,
  }: Props = $props()

  let strip = $state<HTMLDivElement | null>(null)
  let pill = $state({ left: 0, width: 0 })
  let settled = $state(false)

  function measure(active: string) {
    const found = strip?.querySelector<HTMLElement>(
      `[data-option="${CSS.escape(active)}"]`,
    )

    if (!found) {
      pill = { left: pill.left, width: 0 }

      return
    }

    const unit = rem(1)

    pill = { left: found.offsetLeft / unit, width: found.offsetWidth / unit }
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
  role="group"
  aria-label={label}
  class="relative flex gap-1 bg-base-200 p-1 hairline"
>
  <span
    aria-hidden="true"
    class={[
      "absolute top-1 bottom-1 left-0 border-b-2 border-primary bg-base-100",
      "hairline transition-all ease-out",
      settled ? "duration-140" : "duration-0",
      pill.width === 0 && "opacity-0",
    ]}
    style:transform="translateX({pill.left}rem)"
    style:width="{pill.width}rem"
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
      class={[
        "relative flex flex-1 cursor-pointer items-center justify-center gap-2",
        "whitespace-nowrap transition-colors",
        small ? "px-2 py-1 text-xs" : "px-3 py-2 text-sm",
        value === option.value
          ? "font-medium text-base-content"
          : "text-base-content/70 hover:text-base-content",
      ]}
    >
      {#if option.icon}
        <Icon icon={option.icon} class="size-4 shrink-0" />
      {/if}

      {option.label}
    </button>
  {/each}
</div>
