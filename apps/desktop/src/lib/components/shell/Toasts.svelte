<script lang="ts">
  import { fly } from "svelte/transition"

  import { calm, Icon, leave } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  const slide = () => ({ duration: calm() ? 0 : 140, y: 8 })
</script>

{#snippet toast(
  tone: "error" | "notice",
  text: string,
  hint: string,
  dismiss: () => void,
)}
  <div
    in:fly|global={slide()}
    out:leave|global
    role={tone === "error" ? "alert" : "status"}
    class={[
      "floating lift relative flex w-md max-w-full items-start gap-3 py-3",
      "pr-2 pl-4",
    ]}
  >
    <span
      aria-hidden="true"
      class={[
        "absolute inset-y-0 left-0 w-1",
        tone === "error" ? "bg-error" : "bg-primary",
      ]}
    ></span>

    <Icon
      icon={tone === "error" ? "lucide:circle-alert" : "lucide:info"}
      class={[
        "mt-1 size-4 shrink-0",
        tone === "error" ? "text-error" : "text-primary",
      ]}
    />

    <div class="min-w-0 flex-1">
      <p
        class={[
          "max-h-40 overflow-y-auto text-sm wrap-anywhere whitespace-pre-wrap",
          "select-text",
        ]}
      >
        {text}
      </p>

      {#if hint}
        <p class="pt-2 text-sm leading-relaxed text-base-content/80">
          {hint}
        </p>
      {/if}
    </div>

    <button
      type="button"
      aria-label={m.close()}
      onclick={dismiss}
      class="btn btn-square btn-ghost btn-xs shrink-0 text-base-content/70"
    >
      <Icon icon="lucide:x" class="size-4" />
    </button>
  </div>
{/snippet}

{#if workspace.error || workspace.notice}
  <div
    class={[
      "fixed right-4 bottom-4 z-80 flex max-w-full flex-col items-end gap-2",
      "pl-4",
    ]}
  >
    {#if workspace.notice}
      {@render toast("notice", workspace.notice, "", () => {
        workspace.notice = ""
      })}
    {/if}

    {#if workspace.error}
      {@render toast("error", workspace.error, workspace.errorHint, () => {
        workspace.error = ""
      })}
    {/if}
  </div>
{/if}
