<script lang="ts">
  import { Icon } from "../icons"

  type Props = {
    icon: string
    title: string
    detail: string
    on: boolean
    disabled?: boolean
    onclick: () => void
  }

  let { icon, title, detail, on, disabled = false, onclick }: Props = $props()
</script>

<label
  class={[
    "flex items-center gap-4 px-4 py-3 transition-colors",
    disabled
      ? "cursor-not-allowed opacity-60"
      : "cursor-pointer hover:bg-base-content/5",
  ]}
>
  <Icon
    {icon}
    class={["size-4 shrink-0", on ? "text-primary" : "text-base-content/70"]}
  />

  <span class="min-w-0 flex-1">
    <span class="block text-sm">{title}</span>
    <span class="block text-xs text-base-content/70">{detail}</span>
  </span>

  <input
    type="checkbox"
    role="switch"
    checked={on}
    {disabled}
    onchange={event => {
      event.currentTarget.checked = on
      onclick()
    }}
    class="toggle toggle-sm toggle-primary shrink-0"
  />
</label>
