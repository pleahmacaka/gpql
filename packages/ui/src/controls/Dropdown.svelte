<script lang="ts" generics="Value extends string">
  import { Icon } from "../icons"

  type Props = {
    options: { value: Value; label: string }[]
    value: Value
    onpick: (value: Value) => void
    wide?: boolean
    search?: string
    empty?: string
  }

  let {
    options,
    value,
    onpick,
    wide = false,
    search = "",
    empty = "no match",
  }: Props = $props()

  let host = $state<HTMLDetailsElement | null>(null)
  let query = $state("")
  let spot = $state({ left: 0, top: 0, width: 0 })

  // any scrolling ancestor would clip a list positioned inside it, so the list
  // is placed against the viewport instead
  function place() {
    const trigger = host?.querySelector("summary")

    if (!trigger) {
      return
    }

    const box = trigger.getBoundingClientRect()
    const room = window.innerHeight - box.bottom

    spot = {
      left: box.left,
      top: room < 240 && box.top > room ? box.top - Math.min(box.top, 288) : box.bottom + 4,
      width: box.width,
    }
  }

  let current = $derived(options.find(option => option.value === value))

  let shown = $derived(
    query.trim() === ""
      ? options
      : options.filter(option =>
          option.label.toLowerCase().includes(query.trim().toLowerCase()),
        ),
  )

  function pick(next: Value) {
    onpick(next)
    query = ""

    if (host) {
      host.open = false
    }
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "Enter" && shown[0]) {
      event.preventDefault()
      pick(shown[0].value)
    }

    if (event.key === "Escape" && host) {
      event.preventDefault()
      host.open = false
    }
  }
</script>

<svelte:window onresize={() => host?.open && place()} />

<details
  bind:this={host}
  ontoggle={() => {
    query = ""
    place()
  }}
  class="dropdown dropdown-end {wide ? 'w-full' : ''}"
>
  <summary
    class="flex cursor-pointer list-none items-center gap-2 rounded-field
      text-sm marker:content-none
      {wide ? 'w-full py-1' : 'px-2 py-1 hover:bg-base-300'}"
  >
    <span class="{wide ? 'flex-1' : ''} truncate">
      {current?.label ?? value}
    </span>

    <Icon icon="lucide:chevron-down" class="size-4 text-base-content/40" />
  </summary>

  <div
    class="floating fixed z-70 flex max-h-72 flex-col rounded-box bg-base-100
      p-1 lift {wide ? '' : 'w-44'}"
    style:left="{spot.left}px"
    style:top="{spot.top}px"
    style:min-width="{spot.width}px"
  >
    {#if search}
      <div
        class="flex items-center gap-2 border-b border-base-content/8 px-2
          pb-1"
      >
        <Icon icon="lucide:search" class="size-4 text-base-content/35" />

        <input
          bind:value={query}
          onkeydown={keys}
          placeholder={search}
          class="min-w-0 flex-1 bg-transparent py-1 text-sm outline-none
            select-text placeholder:text-base-content/30"
        />
      </div>
    {/if}

    <ul class="min-h-0 flex-1 overflow-y-auto">
      {#each shown as option (option.value)}
        <li>
          <button
            type="button"
            onclick={() => pick(option.value)}
            class="flex w-full items-center gap-2 rounded-field px-2 py-2
              text-left text-sm transition-colors hover:bg-base-200
              {option.value === value ? 'text-primary' : ''}"
          >
            <span class="flex-1 truncate">{option.label}</span>

            {#if option.value === value}
              <Icon icon="lucide:check" class="size-4 shrink-0" />
            {/if}
          </button>
        </li>
      {:else}
        <li class="px-2 py-3 text-center text-xs text-base-content/40">
          {empty}
        </li>
      {/each}
    </ul>
  </div>
</details>
