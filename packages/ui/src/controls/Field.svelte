<script lang="ts">
  type Props = {
    label: string
    value: string
    oninput?: (value: string) => void
    type?: "text" | "password"
    placeholder?: string
    autofocus?: boolean
    suggestions?: string[]
    onkeydown?: (event: KeyboardEvent) => void
  }

  let {
    label,
    value = $bindable(),
    oninput,
    type = "text",
    placeholder = "",
    autofocus = false,
    suggestions = [],
    onkeydown,
  }: Props = $props()

  let focused = $state(false)

  let listId = $derived(
    suggestions.length > 0 ? `list-${label.replace(/\s+/g, "-")}` : undefined,
  )
</script>

<label
  class="block cursor-text rounded-field px-3 pt-1.5 pb-2 transition-colors
    {focused ? 'bg-primary/10' : 'bg-base-200'}"
>
  <span
    class="block text-xs {focused
      ? 'text-primary'
      : 'text-base-content/45'}"
  >
    {label}
  </span>

  {#if type === "password"}
    <input
      type="password"
      bind:value
      oninput={event => oninput?.(event.currentTarget.value)}
      {placeholder}
      {onkeydown}
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
      class="w-full bg-transparent text-sm outline-none placeholder:text-base-content/25"
    />
  {:else}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      type="text"
      list={listId}
      bind:value
      oninput={event => oninput?.(event.currentTarget.value)}
      {placeholder}
      {autofocus}
      {onkeydown}
      spellcheck="false"
      autocapitalize="off"
      autocorrect="off"
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
      class="w-full bg-transparent text-sm outline-none placeholder:text-base-content/25"
    />
  {/if}

  {#if listId}
    <datalist id={listId}>
      {#each suggestions as option (option)}
        <option value={option}></option>
      {/each}
    </datalist>
  {/if}
</label>
