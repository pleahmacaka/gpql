<script lang="ts">
  import { Icon } from "../icons"
  import type { Ending } from "../motion"
  import Dialog from "./Dialog.svelte"

  type Tone = "primary" | "warning" | "error"

  type Props = {
    title: string
    body?: string
    confirm: string
    cancel: string
    alternate?: string
    onalternate?: () => void
    icon?: string
    danger?: boolean
    tone?: Tone
    onconfirm: () => void
    oncancel: () => void
  }

  let {
    title,
    body = "",
    confirm,
    cancel,
    alternate,
    onalternate,
    icon = "lucide:trash-2",
    danger = true,
    tone,
    onconfirm,
    oncancel,
  }: Props = $props()

  let shade = $derived<Tone>(tone ?? (danger ? "error" : "primary"))
  let ending = $state<Ending>("cancel")

  function settle(run: () => void, as: Ending) {
    ending = as
    run()
  }

  const BADGE = {
    primary: "bg-primary/15 text-primary",
    warning: "bg-warning/15 text-warning",
    error: "bg-error/15 text-error",
  }

  const ACTION = {
    primary: "btn-primary",
    warning: "btn-warning",
    error: "btn-error",
  }
</script>

<Dialog
  label={title}
  onclose={oncancel}
  dismiss={cancel}
  size="sm"
  tone={shade}
  role="alertdialog"
  {ending}
>
  <div class="flex gap-4 p-6">
    <span
      class={["grid size-8 shrink-0 place-items-center", BADGE[shade]]}
    >
      <Icon {icon} class="size-4" />
    </span>

    <div class="min-w-0 flex-1">
      <h2 class="text-base font-semibold tracking-tight text-balance">
        {title}
      </h2>

      {#if body}
        <p class="pt-2 text-sm wrap-anywhere text-base-content/70">{body}</p>
      {/if}
    </div>
  </div>

  <div
    class="flex justify-end gap-2 border-t border-base-content/10 px-6 py-4"
  >
    <button
      type="button"
      data-autofocus={shade === "primary" ? undefined : ""}
      onclick={() => settle(oncancel, "cancel")}
      class="btn btn-ghost btn-sm font-medium"
    >
      {cancel}
    </button>

    {#if alternate && onalternate}
      <button
        type="button"
        onclick={() => settle(onalternate, "confirm")}
        class="btn btn-soft btn-error btn-sm font-medium"
      >
        {alternate}
      </button>
    {/if}

    <button
      type="button"
      data-autofocus={shade === "primary" ? "" : undefined}
      onclick={() => settle(onconfirm, "confirm")}
      class={["btn btn-sm font-medium", ACTION[shade]]}
    >
      {confirm}
    </button>
  </div>
</Dialog>
