<script lang="ts">
  type Props = {
    label: string
    value: string
    oninput?: (value: string) => void
    type?: "text" | "password"
    placeholder?: string
    hint?: string
    invalid?: boolean
    autofocus?: boolean
    onkeydown?: (event: KeyboardEvent) => void
  }

  let {
    label,
    value = $bindable(),
    oninput,
    type = "text",
    placeholder = "",
    hint = "",
    invalid = false,
    autofocus = false,
    onkeydown,
  }: Props = $props()

  function focus(node: HTMLInputElement) {
    if (autofocus) {
      node.focus()
    }
  }
</script>

<label class="group flex min-w-0 flex-col gap-1">
  <span
    class={[
      "text-xs transition-colors group-focus-within:text-primary",
      invalid ? "text-error" : "text-base-content/70",
    ]}
  >
    {label}
  </span>

  <input
    {type}
    bind:value
    oninput={event => oninput?.(event.currentTarget.value)}
    {placeholder}
    {onkeydown}
    aria-invalid={invalid}
    spellcheck="false"
    autocapitalize="off"
    autocomplete="off"
    {@attach focus}
    class={[
      "input input-sm w-full bg-base-100 text-sm",
      "placeholder:text-base-content/60",
      invalid && "input-error",
    ]}
  />

  {#if hint}
    <span
      class={["text-xs", invalid ? "text-error" : "text-base-content/70"]}
    >
      {hint}
    </span>
  {/if}
</label>
