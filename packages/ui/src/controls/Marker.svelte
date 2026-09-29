<script lang="ts">
  type Props = {
    label: string
    tone?: "primary" | "muted" | "success" | "warning" | "error"
    as?: "p" | "h2" | "h3" | "span"
    id?: string
    class?: string
  }

  let {
    label,
    tone = "primary",
    as = "p",
    id,
    class: extra = "",
  }: Props = $props()

  const SQUARE = {
    primary: "bg-primary",
    muted: "bg-base-content/40",
    success: "bg-success",
    warning: "bg-warning",
    error: "bg-error",
  }
</script>

<svelte:element
  this={as}
  {id}
  class={[
    "flex items-center gap-2 text-xs font-medium text-base-content/70",
    extra,
  ]}
>
  <span aria-hidden="true" class={["size-2 shrink-0", SQUARE[tone]]}></span>
  {label}
</svelte:element>
