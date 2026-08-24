<script lang="ts">
  import type { Snippet } from "svelte"

  import { Icon } from "./icons"

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

  const tabs = [
    { label: "Data", icon: "lucide:table-2" },
    { label: "Query", icon: "lucide:terminal" },
    { label: "Schema", icon: "lucide:git-fork" },
  ]
</script>

<header
  data-tauri-drag-region
  class="flex h-11 shrink-0 items-center gap-2 pl-4"
>
  <span class="text-sm text-base-content/45">GPQL</span>

  <svelte:element
    this={live ? "button" : "span"}
    role={live ? "button" : undefined}
    tabindex={live ? 0 : undefined}
    onclick={onchip}
    class="flex items-center gap-2 rounded-field px-2 py-1 text-sm
      {live ? 'transition-colors hover:bg-base-300' : ''}"
  >
    <Icon icon={chipIcon} class="size-4 text-base-content/50" />
    <span>{chip}</span>
    <Icon icon="lucide:chevron-down" class="size-3.5 text-base-content/40" />
  </svelte:element>

  <div data-tauri-drag-region class="flex flex-1 justify-center">
    {#if tab}
      <div class="flex gap-1 rounded-selector bg-base-300/60 p-1">
        {#each tabs as entry (entry.label)}
          <svelte:element
            this={live ? "button" : "span"}
            role={live ? "button" : undefined}
            tabindex={live ? 0 : undefined}
            onclick={() => ontab?.(entry.label)}
            aria-pressed={entry.label === tab}
            class="flex items-center gap-1.5 rounded-selector px-3 py-1 text-sm
              transition-colors {entry.label === tab
              ? 'bg-base-100 font-medium hairline'
              : 'text-base-content/55 hover:text-base-content'}"
          >
            <Icon icon={entry.icon} class="size-3.5" />
            {entry.label}
          </svelte:element>
        {/each}
      </div>
    {/if}
  </div>

  <svelte:element
    this={live ? "button" : "span"}
    role={live ? "button" : undefined}
    tabindex={live ? 0 : undefined}
    aria-label="Agent"
    aria-pressed={agentOn}
    onclick={onagent}
    class="grid h-11 w-12 place-items-center
      {agentOn ? 'text-primary' : 'text-base-content/60'}
      {live ? 'transition-colors hover:bg-base-300' : ''}"
  >
    <Icon icon="lucide:sparkles" class="size-4" />
  </svelte:element>

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
