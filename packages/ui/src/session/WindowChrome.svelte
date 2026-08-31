<script lang="ts">
  import type { Snippet } from "svelte"

  import { Icon } from "../icons"
  import Logo from "../controls/Logo.svelte"

  type Props = {
    chip: string
    chipIcon?: string
    tab?: string | null
    live?: boolean
    onchip?: () => void
    ontab?: (tab: string) => void
    onsettings?: () => void
    onagent?: () => void
    agentOn?: boolean
    controls?: Snippet
  }

  let {
    chip,
    chipIcon = "lucide:database",
    tab = null,
    live = false,
    onchip,
    ontab,
    onsettings,
    onagent,
    agentOn = false,
    controls,
  }: Props = $props()

  let strip = $state<HTMLDivElement | null>(null)
  let pill = $state({ left: 0, width: 0 })
  let settled = $state(false)

  const tabs = [
    { label: "Data", icon: "lucide:table-2" },
    { label: "Query", icon: "lucide:terminal" },
    { label: "Schema", icon: "lucide:git-fork" },
  ]

  function measure(active: string) {
    const found = strip?.querySelector<HTMLElement>(`[data-tab="${active}"]`)

    if (!found) {
      return
    }

    pill = { left: found.offsetLeft, width: found.offsetWidth }
  }

  $effect(() => {
    const active = tab

    if (!strip || !active) {
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
    if (!strip || !tab) {
      return
    }

    const watcher = new ResizeObserver(() => measure(tab))

    watcher.observe(strip)

    return () => watcher.disconnect()
  })
</script>

<header
  data-tauri-drag-region
  class="flex h-11 shrink-0 items-center gap-2 pl-4"
>
  <span class="flex items-center gap-2 text-sm text-base-content/45">
    <Logo class="size-4 text-base-content/70" plain />
    GPQL
  </span>

  <svelte:element
    this={live ? "button" : "span"}
    role={live ? "button" : undefined}
    tabindex={live ? 0 : undefined}
    onclick={onchip}
    class="flex items-center gap-2 rounded-field px-2 py-1 text-sm
      {live ? 'transition-colors hover:bg-base-300' : ''}"
  >
    <Icon
      icon={chipIcon}
      class="size-4 stroke-current stroke-1 text-base-content/50"
    />
    <span class="whitespace-nowrap">{chip}</span>
    <Icon icon="lucide:chevron-down" class="size-4 text-base-content/40" />
  </svelte:element>

  <div data-tauri-drag-region class="flex flex-1 justify-center">
    {#if tab}
      <div
        bind:this={strip}
        class="relative flex gap-1 rounded-selector bg-base-300/60 p-1"
      >
        <span
          aria-hidden="true"
          class="absolute top-1 bottom-1 left-0 rounded-selector bg-base-100 hairline transition-all duration-200 ease-out {settled ? '' : 'duration-0'}"
          style:transform="translateX({pill.left}px)"
          style:width="{pill.width}px"
        ></span>

        {#each tabs as entry (entry.label)}
          <svelte:element
            this={live ? "button" : "span"}
            role={live ? "button" : undefined}
            tabindex={live ? 0 : undefined}
            onclick={() => ontab?.(entry.label)}
            aria-pressed={entry.label === tab}
            data-tab={entry.label}
            class="relative z-10 flex items-center gap-2 rounded-selector px-3
              py-1 text-sm transition-colors {entry.label === tab
              ? 'font-medium'
              : 'text-base-content/55 hover:text-base-content'}"
          >
            <Icon icon={entry.icon} class="size-4" />
            {entry.label}
          </svelte:element>
        {/each}
      </div>
    {/if}
  </div>

  {#if onagent}
    <button
      type="button"
      aria-label="Agent"
      aria-pressed={agentOn}
      onclick={onagent}
      class="grid h-11 w-12 place-items-center transition-colors
        hover:bg-base-300
        {agentOn ? 'text-primary' : 'text-base-content/60'}"
    >
      <Icon icon="lucide:sparkles" class="size-4" />
    </button>
  {/if}

  <svelte:element
    this={live ? "button" : "span"}
    role={live ? "button" : undefined}
    tabindex={live ? 0 : undefined}
    aria-label="Settings"
    onclick={onsettings}
    class="grid h-11 w-12 place-items-center text-base-content/60
      {live ? 'transition-colors hover:bg-base-300' : ''}"
  >
    <Icon icon="lucide:settings" class="size-4" />
  </svelte:element>

  {#if controls}
    {@render controls()}
  {:else}
    {#each ["lucide:minus", "lucide:square", "lucide:x"] as control (control)}
      <span class="grid h-11 w-9 place-items-center text-base-content/50">
        <Icon
          icon={control}
          class={control === "lucide:square" ? "size-3" : "size-4"}
        />
      </span>
    {/each}
  {/if}
</header>
