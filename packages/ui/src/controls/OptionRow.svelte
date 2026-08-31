<script lang="ts">
  import { Icon } from "../icons"

  type Props = {
    icon: string
    title: string
    detail: string
    on: boolean
    onclick: () => void
  }

  let { icon, title, detail, on, onclick }: Props = $props()
</script>

<button
  type="button"
  {onclick}
  aria-pressed={on}
  class="flex w-full items-center gap-3 rounded-field px-3 py-3 text-left
    transition-colors {on
    ? 'bg-primary/10 hover:bg-primary/15'
    : 'bg-base-200 hover:bg-base-300'}"
>
  <Icon
    {icon}
    class="size-4 {on ? 'text-accent' : 'text-base-content/40'}"
  />

  <span class="flex-1">
    <span class="block text-sm {on ? 'text-primary' : ''}">{title}</span>
    <span class="block text-xs text-base-content/45">{detail}</span>
  </span>

  <Icon
    icon={on ? "lucide:check" : "lucide:x"}
    class="size-4 {on ? 'text-primary' : 'text-base-content/30'}"
  />
</button>
