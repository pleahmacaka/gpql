<script lang="ts">
  import { Icon } from "../icons"

  type Props = {
    options: { value: string; label: string }[]
    value: string
    onpick: (value: string) => void
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

  let current = $derived(options.find(option => option.value === value))

  let shown = $derived(
    query.trim() === ""
      ? options
      : options.filter(option =>
          option.label.toLowerCase().includes(query.trim().toLowerCase()),
        ),
  )

  function pick(next: string) {
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

<details
  bind:this={host}
  ontoggle={() => (query = "")}
  class="dropdown dropdown-end {wide ? 'w-full' : ''}"
>
  <summary
    class="flex cursor-pointer list-none items-center gap-1.5 rounded-field
      text-sm marker:content-none
      {wide ? 'w-full py-0.5' : 'px-2 py-1 hover:bg-base-300'}"
  >
    <span class="{wide ? 'flex-1' : ''} truncate">
      {current?.label ?? value}
    </span>

    <Icon icon="lucide:chevron-down" class="size-3.5 text-base-content/40" />
  </summary>

  <div
    class="dropdown-content floating z-50 mt-1 flex max-h-72 flex-col
      rounded-box bg-base-100 p-1 lift {wide ? 'w-max min-w-full' : 'w-44'}"
  >
    {#if search}
      <div
        class="flex items-center gap-1.5 border-b border-base-content/8 px-2
          pb-1"
      >
        <Icon icon="lucide:search" class="size-3.5 text-base-content/35" />

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
            class="flex w-full items-center gap-2 rounded-field px-2 py-1.5
              text-left text-sm transition-colors hover:bg-base-200
              {option.value === value ? 'text-primary' : ''}"
          >
            <span class="flex-1 truncate">{option.label}</span>

            {#if option.value === value}
              <Icon icon="lucide:check" class="size-3.5 shrink-0" />
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
