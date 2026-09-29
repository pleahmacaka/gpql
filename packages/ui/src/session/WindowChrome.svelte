<script lang="ts">
  import type { Snippet } from "svelte"

  import Keycap from "../controls/Keycap.svelte"
  import Logo from "../controls/Logo.svelte"
  import { rem } from "../controls/rem"
  import { Icon } from "../icons"

  type Props = {
    chip: string
    chipIcon?: string
    chipOpen?: boolean
    tone?: "idle" | "live" | "warning"
    tab?: string | null
    live?: boolean
    onchip?: () => void
    ontab?: (tab: string) => void
    onsettings?: () => void
    onpalette?: () => void
    onagent?: () => void
    agentOn?: boolean
    status?: Snippet
    controls?: Snippet
    labels?: Partial<
      Record<
        "data" | "query" | "schema" | "agent" | "settings" | "palette" | "tabs",
        string
      >
    >
  }

  let {
    chip,
    chipIcon = "lucide:database",
    chipOpen = false,
    tone = "live",
    tab = null,
    live = false,
    onchip,
    ontab,
    onsettings,
    onpalette,
    onagent,
    agentOn = false,
    status,
    controls,
    labels = {},
  }: Props = $props()

  let strip = $state<HTMLDivElement | null>(null)
  let bar = $state({ left: 0, width: 0 })
  let settled = $state(false)

  let tabs = $derived([
    { id: "data", label: labels.data ?? "Data", icon: "lucide:table-2" },
    { id: "query", label: labels.query ?? "Query", icon: "lucide:terminal" },
    { id: "schema", label: labels.schema ?? "Schema", icon: "lucide:git-fork" },
  ])

  let active = $derived(tab?.toLowerCase() ?? null)

  const SQUARE = {
    idle: "bg-base-content/40",
    live: "bg-success",
    warning: "bg-warning",
  }

  function measure(current: string) {
    const found = strip?.querySelector<HTMLElement>(`[data-tab="${current}"]`)

    if (!found) {
      return
    }

    const unit = rem(1)

    bar = { left: found.offsetLeft / unit, width: found.offsetWidth / unit }
  }

  function arrows(event: KeyboardEvent) {
    const at = tabs.findIndex(entry => entry.id === active)
    const by =
      event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0

    if (by === 0 || at === -1) {
      return
    }

    event.preventDefault()

    const next = tabs[(at + by + tabs.length) % tabs.length]

    ontab?.(next.id)
    strip?.querySelector<HTMLElement>(`[data-tab="${next.id}"]`)?.focus()
  }

  $effect(() => {
    const current = active

    if (!strip || !current) {
      return
    }

    measure(current)

    if (settled) {
      return
    }

    const frame = requestAnimationFrame(() => {
      measure(current)
      settled = true
    })

    return () => cancelAnimationFrame(frame)
  })

  $effect(() => {
    const current = active

    if (!strip || !current) {
      return
    }

    const watcher = new ResizeObserver(() => measure(current))

    watcher.observe(strip)

    return () => watcher.disconnect()
  })
</script>

<header
  data-tauri-drag-region
  class={[
    "relative flex h-10 shrink-0 items-stretch border-b",
    "border-base-content/10",
  ]}
>
  <span
    data-tauri-drag-region
    class="flex items-center gap-2 pr-4 pl-4 text-sm font-semibold"
  >
    <Logo class="size-4 text-base-content" plain />
    GPQL
  </span>

  {#snippet chipFace()}
    <span aria-hidden="true" class={["size-2 shrink-0", SQUARE[tone]]}></span>
    <Icon icon={chipIcon} class="size-4 shrink-0 text-base-content/70" />
    <span class="max-w-60 truncate font-medium">{chip}</span>
    <Icon
      icon="lucide:chevron-down"
      class={[
        "size-4 shrink-0 text-base-content/70 transition-transform",
        chipOpen && "rotate-180",
      ]}
    />
  {/snippet}

  {#if onchip}
    <button
      type="button"
      data-session-chip
      aria-haspopup={live ? "dialog" : undefined}
      aria-expanded={live ? chipOpen : undefined}
      onclick={onchip}
      class={[
        "my-1 flex min-w-0 cursor-pointer items-center gap-2 border-l px-3",
        "border-base-content/10 text-sm transition-colors",
        "hover:bg-base-content/5",
        chipOpen && "hud hud-small hud-lit bg-base-content/5",
      ]}
    >
      {@render chipFace()}
    </button>
  {:else}
    <span
      data-session-chip
      class={[
        "my-1 flex min-w-0 items-center gap-2 border-l border-base-content/10",
        "px-3 text-sm",
      ]}
    >
      {@render chipFace()}
    </span>
  {/if}

  <div
    data-tauri-drag-region
    class="@container flex min-w-32 flex-1 justify-center"
  >
    {#if tab}
      <div
        bind:this={strip}
        role={ontab ? "tablist" : undefined}
        aria-label={ontab ? (labels.tabs ?? "Views") : undefined}
        onkeydown={ontab ? arrows : undefined}
        class="relative flex h-full"
      >
        <span
          aria-hidden="true"
          class={[
            "absolute bottom-0 left-0 border-b-2 border-primary",
            "transition-all ease-out",
            settled ? "duration-140" : "duration-0",
          ]}
          style:transform="translateX({bar.left}rem)"
          style:width="{bar.width}rem"
        ></span>

        {#each tabs as entry, index (entry.id)}
          <svelte:element
            this={ontab ? "button" : "span"}
            type={ontab ? "button" : undefined}
            role={ontab ? "tab" : undefined}
            tabindex={ontab ? (entry.id === active ? 0 : -1) : undefined}
            aria-selected={ontab ? entry.id === active : undefined}
            aria-keyshortcuts={ontab && live ? String(index + 1) : undefined}
            onclick={ontab ? () => ontab(entry.id) : undefined}
            data-tab={entry.id}
            class={[
              "relative flex items-center gap-2 px-3 text-sm whitespace-nowrap",
              "transition-colors @sm:px-4",
              ontab && "cursor-pointer",
              entry.id === active
                ? "font-medium text-base-content"
                : "text-base-content/70 hover:text-base-content",
            ]}
          >
            <Icon icon={entry.icon} class="size-4 shrink-0" />
            <span class="sr-only @xs:not-sr-only">{entry.label}</span>
          </svelte:element>
        {/each}
      </div>
    {/if}
  </div>

  {#if status}
    <div class="flex items-center gap-2 px-2 whitespace-nowrap">
      {@render status()}
    </div>
  {/if}

  {#if onpalette}
    <button
      type="button"
      aria-label={labels.palette ?? "Quick actions"}
      aria-keyshortcuts="Control+K"
      onclick={onpalette}
      class={[
        "flex cursor-pointer items-center gap-2 px-3 text-base-content/70",
        "transition-colors hover:bg-base-content/5 hover:text-base-content",
      ]}
    >
      <Icon icon="lucide:search" class="size-4" />
      <Keycap keys={["ctrl", "k"]} />
    </button>
  {/if}

  {#if onagent}
    <button
      type="button"
      aria-label={labels.agent ?? "Agent"}
      aria-pressed={agentOn}
      onclick={onagent}
      class={[
        "grid w-10 cursor-pointer place-items-center transition-colors",
        "hover:bg-base-content/5",
        agentOn ? "text-primary" : "text-base-content/70",
      ]}
    >
      <Icon icon="lucide:sparkles" class="size-4" />
    </button>
  {/if}

  {#if onsettings}
    <button
      type="button"
      aria-label={labels.settings ?? "Settings"}
      aria-keyshortcuts={live ? "Control+Comma" : undefined}
      onclick={onsettings}
      class={[
        "grid w-10 cursor-pointer place-items-center text-base-content/70",
        "transition-colors hover:bg-base-content/5 hover:text-base-content",
      ]}
    >
      <Icon icon="lucide:settings" class="size-4" />
    </button>
  {/if}

  {#if controls}
    {@render controls()}
  {:else}
    {#each ["lucide:minus", "lucide:square", "lucide:x"] as control (control)}
      <span class="grid w-10 place-items-center text-base-content/70">
        <Icon
          icon={control}
          class={control === "lucide:square" ? "size-3" : "size-4"}
        />
      </span>
    {/each}
  {/if}
</header>
