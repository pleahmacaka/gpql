<script lang="ts">
  import { Icon } from "../icons"
  import { type Ending, leave, rise } from "../motion"
  import { rem } from "./rem"

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

  let unit = rem(1)
  let left = $derived(Math.min(x, window.innerWidth - rem(14)) / unit)
  let top = $derived(
    Math.min(y, window.innerHeight - items.length * rem(2.25) - rem(1)) / unit,
  )

  let ending = $state<Ending>("cancel")

  function pick(item: MenuItem) {
    ending = "confirm"
    onclose()
    item.run()
  }

  function start(node: HTMLElement) {
    node.querySelector("button")?.focus()
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault()
      event.stopPropagation()
      onclose()

      return
    }

    const menu = event.currentTarget

    if (!(menu instanceof HTMLElement)) {
      return
    }

    const all = [...menu.querySelectorAll<HTMLButtonElement>("button")]
    const at = all.findIndex(item => item === document.activeElement)
    const moves: Record<string, number> = {
      ArrowDown: (at + 1) % all.length,
      ArrowUp: (at - 1 + all.length) % all.length,
      Home: 0,
      End: all.length - 1,
    }

    if (event.key in moves) {
      event.preventDefault()
      all[moves[event.key]]?.focus()
    }
  }
</script>

<svelte:window onblur={onclose} />

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
  in:rise
  out:leave={{ as: ending }}
  role="menu"
  tabindex="-1"
  onkeydown={keys}
  {@attach start}
  class="floating lift fixed z-70 w-56 p-1 outline-none"
  style:left="{left}rem"
  style:top="{top}rem"
>
  {#each items as item, index (index)}
    <li role="none">
      <button
        type="button"
        role="menuitem"
        onclick={() => pick(item)}
        class={[
          "flex w-full cursor-pointer items-center gap-2 px-2 py-2 text-left",
          "text-sm transition-colors hover:bg-base-content/5",
          "focus-visible:bg-base-content/5 focus-visible:outline-offset-0",
          item.danger && "text-error",
        ]}
      >
        {#if item.icon}
          <Icon icon={item.icon} class="size-4 shrink-0 opacity-70" />
        {:else}
          <span class="size-4 shrink-0"></span>
        {/if}

        <span class="truncate">{item.label}</span>
      </button>
    </li>
  {/each}
</menu>
