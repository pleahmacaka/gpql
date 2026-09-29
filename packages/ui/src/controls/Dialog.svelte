<script lang="ts">
  import type { Snippet } from "svelte"
  import { fade } from "svelte/transition"

  import { type Ending, leave, rise, veil } from "../motion"
  import { trap } from "./trap"

  type Props = {
    label: string
    onclose: () => void
    dismiss?: string
    size?: "sm" | "md" | "lg" | "xl"
    place?: "center" | "top"
    tone?: "primary" | "warning" | "error"
    role?: "dialog" | "alertdialog"
    ending?: Ending
    class?: string
    children: Snippet
  }

  let {
    label,
    onclose,
    dismiss = "Close",
    size = "md",
    place = "center",
    tone = "primary",
    role = "dialog",
    ending = "cancel",
    class: extra = "",
    children,
  }: Props = $props()

  const SIZE = {
    sm: "max-w-sm",
    md: "max-w-lg",
    lg: "max-w-2xl",
    xl: "max-w-4xl",
  }

  const TONE = {
    primary: "",
    warning: "hud-warning",
    error: "hud-error",
  }
</script>

<div
  class={[
    "fixed inset-0 z-70 flex justify-center p-6",
    place === "top" ? "items-start pt-24" : "items-center",
  ]}
>
  <button
    type="button"
    tabindex="-1"
    aria-label={dismiss}
    transition:fade|global={veil()}
    onclick={onclose}
    class="scrim absolute inset-0 cursor-default"
  ></button>

  <div
    in:rise|global
    out:leave|global={{ as: ending }}
    use:trap={onclose}
    {role}
    aria-modal="true"
    aria-label={label}
    tabindex="-1"
    class={[
      "hud hud-lit floating lift relative flex max-h-full w-full flex-col",
      "outline-none",
      SIZE[size],
      TONE[tone],
      extra,
    ]}
  >
    {@render children()}
  </div>
</div>
