<script lang="ts" generics="Value extends string">
  import { Icon } from "../icons"
  import { type Ending, leave, rise } from "../motion"
  import { rem } from "./rem"

  type Props = {
    options: { value: Value; label: string }[]
    value: Value
    onpick: (value: Value) => void
    wide?: boolean
    small?: boolean
    label?: string
    search?: string
    empty?: string
  }

  let {
    options,
    value,
    onpick,
    wide = false,
    small = false,
    label,
    search = "",
    empty = "no match",
  }: Props = $props()

  let host = $state<HTMLDivElement | null>(null)
  let trigger = $state<HTMLButtonElement | null>(null)
  let list = $state<HTMLUListElement | null>(null)
  let popup = $state<HTMLDivElement | null>(null)
  let open = $state(false)
  let ending = $state<Ending>("cancel")
  let query = $state("")
  let spot = $state({ left: 0, top: 0, width: 0 })

  let current = $derived(options.find(option => option.value === value))

  let shown = $derived(
    query.trim() === ""
      ? options
      : options.filter(option =>
          option.label.toLowerCase().includes(query.trim().toLowerCase()),
        ),
  )

  // placed against the viewport so a scrolling ancestor cannot clip the list
  function place() {
    if (!trigger) {
      return
    }

    const box = trigger.getBoundingClientRect()
    const unit = rem(1)
    const room = window.innerHeight - box.bottom
    const above = room < rem(15) && box.top > room

    spot = {
      left: box.left / unit,
      top: above
        ? (box.top - Math.min(box.top, rem(18))) / unit
        : box.bottom / unit + 0.25,
      width: box.width / unit,
    }
  }

  function show() {
    place()
    query = ""
    ending = "cancel"
    open = true
  }

  function close(as: Ending = "cancel") {
    const held = popup?.contains(document.activeElement) ?? false

    ending = as
    open = false

    if (held) {
      trigger?.focus()
    }
  }

  function pick(next: Value) {
    onpick(next)
    close("confirm")
  }

  function items() {
    return [...(list?.querySelectorAll<HTMLButtonElement>("button") ?? [])]
  }

  function step(event: KeyboardEvent) {
    const all = items()
    const at = all.findIndex(item => item === document.activeElement)
    const moves: Record<string, number> = {
      ArrowDown: at + 1,
      ArrowUp: at - 1,
      Home: 0,
      End: all.length - 1,
    }

    if (event.key === "Escape") {
      event.preventDefault()
      event.stopPropagation()
      close()

      return
    }

    if (event.key === "Tab") {
      event.preventDefault()
      close()

      return
    }

    if (!(event.key in moves) || all.length === 0) {
      return
    }

    event.preventDefault()
    all[Math.max(0, Math.min(all.length - 1, moves[event.key]))]?.focus()
  }

  function opener(event: KeyboardEvent) {
    if (event.key === "Escape" && open) {
      event.preventDefault()
      event.stopPropagation()
      close()

      return
    }

    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault()

      if (!open) {
        show()
      }

      requestAnimationFrame(() => items()[0]?.focus())
    }
  }

  function typed(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.isComposing && shown[0]) {
      event.preventDefault()
      pick(shown[0].value)

      return
    }

    step(event)
  }

  function focusSearch(node: HTMLInputElement) {
    node.focus()
  }

  $effect(() => {
    if (!open) {
      return
    }

    const outside = (event: PointerEvent) => {
      if (!(event.target instanceof Node && host?.contains(event.target))) {
        close()
      }
    }

    const scrolled = (event: Event) => {
      if (!(event.target instanceof Node && list?.contains(event.target))) {
        close()
      }
    }

    window.addEventListener("pointerdown", outside, true)
    window.addEventListener("scroll", scrolled, true)
    window.addEventListener("resize", place)

    return () => {
      window.removeEventListener("pointerdown", outside, true)
      window.removeEventListener("scroll", scrolled, true)
      window.removeEventListener("resize", place)
    }
  })
</script>

<div bind:this={host} class={["relative", wide ? "w-full" : "inline-flex"]}>
  <button
    bind:this={trigger}
    type="button"
    onclick={() => (open ? close() : show())}
    onkeydown={opener}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={label ? `${label}, ${current?.label ?? value}` : undefined}
    class={[
      "flex cursor-pointer items-center gap-2 text-left transition-colors",
      small ? "text-xs" : "text-sm",
      wide
        ? "input input-sm w-full bg-base-100"
        : "px-2 py-1 hover:bg-base-content/5",
    ]}
  >
    <span class={["truncate", wide && "flex-1"]}>
      {current?.label ?? value}
    </span>

    <Icon
      icon="lucide:chevron-down"
      class={[
        "shrink-0 text-base-content/70 transition-transform",
        small ? "size-3" : "size-4",
        open && "rotate-180",
      ]}
    />
  </button>

  {#if open}
    <div
      bind:this={popup}
      in:rise
      out:leave={{ as: ending }}
      class={[
        "floating lift fixed z-70 flex max-h-72 origin-top flex-col p-1",
        !wide && "w-44",
      ]}
      style:left="{spot.left}rem"
      style:top="{spot.top}rem"
      style:min-width="{spot.width}rem"
    >
      {#if search}
        <div
          class={[
            "flex items-center gap-2 border-b border-base-content/10 px-2",
            "pb-1",
          ]}
        >
          <Icon icon="lucide:search" class="size-4 text-base-content/70" />

          <input
            bind:value={query}
            onkeydown={typed}
            placeholder={search}
            aria-label={search}
            {@attach focusSearch}
            class={[
              "min-w-0 flex-1 bg-transparent py-1 text-sm outline-none",
              "select-text placeholder:text-base-content/60",
            ]}
          />
        </div>
      {/if}

      <ul
        bind:this={list}
        role="listbox"
        aria-label={label}
        class="min-h-0 flex-1 overflow-y-auto"
      >
        {#each shown as option, index (index)}
          <li>
            <button
              type="button"
              role="option"
              aria-selected={option.value === value}
              onclick={() => pick(option.value)}
              onkeydown={step}
              class={[
                "flex w-full cursor-pointer items-center gap-2 px-2 py-2",
                "text-left text-sm transition-colors hover:bg-base-content/5",
                "focus-visible:bg-base-content/5",
                "focus-visible:outline-offset-0",
                option.value === value && "text-primary",
              ]}
            >
              <span class="flex-1 truncate">{option.label}</span>

              {#if option.value === value}
                <Icon icon="lucide:check" class="size-4 shrink-0" />
              {/if}
            </button>
          </li>
        {:else}
          <li class="px-2 py-3 text-center text-xs text-base-content/70">
            {empty}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
