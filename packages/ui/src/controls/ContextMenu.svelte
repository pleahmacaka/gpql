<script lang="ts">
  import { scale } from "svelte/transition"

  import { Icon } from "../icons"
  import { pop } from "../motion"

  export type MenuItem = {
    label: string
    icon?: string
    danger?: boolean
    run: () => void
  }

  type Props = {
    x: number
    y: number
    items: MenuItem[]
    onclose: () => void
  }

  let { x, y, items, onclose }: Props = $props()

  let left = $derived(Math.min(x, window.innerWidth - 220))
  let top = $derived(Math.min(y, window.innerHeight - items.length * 34 - 16))

  function pick(item: MenuItem) {
    onclose()
    item.run()
  }
</script>

<svelte:window
  onkeydown={event => event.key === "Escape" && onclose()}
  onblur={onclose}
/>

<div
  class="fixed inset-0 z-70"
  role="presentation"
  oncontextmenu={event => {
    event.preventDefault()
    onclose()
  }}
  onclick={onclose}
></div>

<menu
  transition:scale={pop()}
  class="fixed z-70 w-52 rounded-box floating p-1 lift"
  style:left="{left}px"
  style:top="{top}px"
>
  {#each items as item (item.label)}
    <li>
      <button
        type="button"
        onclick={() => pick(item)}
        class="flex w-full items-center gap-2 rounded-field px-2 py-2 text-left
          text-sm hover:bg-base-200
          {item.danger ? 'text-error' : ''}"
      >
        {#if item.icon}
          <Icon icon={item.icon} class="size-4 shrink-0 opacity-60" />
        {:else}
          <span class="size-4 shrink-0"></span>
        {/if}

        <span class="truncate">{item.label}</span>
      </button>
    </li>
  {/each}
</menu>
