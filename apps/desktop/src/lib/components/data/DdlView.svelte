<script lang="ts">
  import { fade } from "svelte/transition"

  import { EmptyState, Icon, tooltip, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { ObjectKind } from "$lib/types"

  import SqlLines from "./SqlLines.svelte"

  const SKELETON = ["w-64", "w-48", "w-72", "w-40", "w-56", "w-32"]

  const ICONS: Record<ObjectKind, string> = {
    view: "lucide:eye",
    index: "lucide:list-tree",
    sequence: "lucide:hash",
    routine: "lucide:square-function",
    trigger: "lucide:zap",
    type: "lucide:shapes",
  }

  const KINDS: Record<ObjectKind, () => string> = {
    view: m.kind_views,
    index: m.kind_indexes,
    sequence: m.kind_sequences,
    routine: m.kind_routines,
    trigger: m.kind_triggers,
    type: m.kind_types,
  }

  let shown = $derived(workspace.ddl)
</script>

{#if shown}
  <header
    class={[
      "flex h-12 shrink-0 items-center gap-3 border-b border-base-content/10",
      "pr-2 pl-2",
    ]}
  >
    <button
      type="button"
      onclick={() => (workspace.ddl = null)}
      use:tooltip={m.back()}
      aria-label={m.back()}
      class="btn btn-square btn-ghost btn-sm"
    >
      <Icon icon="lucide:arrow-left" class="size-4" />
    </button>

    <Icon
      icon={shown.kind ? ICONS[shown.kind] : "lucide:file-code-2"}
      class="size-4 shrink-0 text-base-content/60"
    />

    <h2 class="min-w-0 truncate text-sm font-semibold">{shown.name}</h2>

    {#if shown.kind}
      <span class="badge badge-sm badge-soft shrink-0">
        {KINDS[shown.kind]()}
      </span>
    {/if}

    <span class="flex-1"></span>

    <button
      type="button"
      disabled={shown.text === ""}
      onclick={() => navigator.clipboard.writeText(shown.text)}
      class="btn btn-soft btn-sm"
    >
      <Icon icon="lucide:copy" class="size-4" />
      {m.menu_copy()}
    </button>
  </header>

  <div
    class="min-h-0 flex-1 overflow-auto py-3"
    style:scrollbar-gutter="stable"
  >
    {#if workspace.ddlLoading}
      <div aria-hidden="true" class="flex flex-col gap-3 px-4 pt-1">
        {#each SKELETON as size, index (index)}
          <span class={["skeleton h-2 opacity-60", size]}></span>
        {/each}
      </div>
    {:else if shown.text === ""}
      <div class="grid h-full place-items-center">
        <EmptyState art="sheet" title={m.ddl_empty()} />
      </div>
    {:else}
      <div in:fade|local={veil()} class="bg-base-100">
        <SqlLines text={shown.text} class="bg-base-100" />
      </div>
    {/if}
  </div>
{/if}
