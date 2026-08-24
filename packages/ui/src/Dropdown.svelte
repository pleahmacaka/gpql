<script lang="ts">
  import { Icon } from "./icons"

  type Props = {
    options: { value: string; label: string }[]
    value: string
    onpick: (value: string) => void
    wide?: boolean
  }

  let { options, value, onpick, wide = false }: Props = $props()

  let host = $state<HTMLDetailsElement | null>(null)

  let current = $derived(options.find(option => option.value === value))

  function pick(next: string) {
    onpick(next)

    if (host) {
      host.open = false
    }
  }
</script>

<details
  bind:this={host}
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

  <ul
    class="dropdown-content z-50 mt-1 max-h-64 overflow-y-auto rounded-box
      bg-base-100 p-1 lift {wide ? 'w-full' : 'w-44'}"
  >
    {#each options as option (option.value)}
      <li>
        <button
          type="button"
          onclick={() => pick(option.value)}
          class="flex w-full items-center gap-2 rounded-field px-2 py-1.5
            text-left text-sm transition-colors hover:bg-base-200
            {option.value === value ? 'text-primary' : ''}"
        >
          <span class="flex-1">{option.label}</span>

          {#if option.value === value}
            <Icon icon="lucide:check" class="size-3.5" />
          {/if}
        </button>
      </li>
    {/each}
  </ul>
</details>
