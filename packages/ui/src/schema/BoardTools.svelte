<script lang="ts">
  import { Icon } from "../icons"
  import { leave, rise } from "../motion"
  import type { TableGroup, Words } from "./board.svelte"

  type Props = {
    words: Words
    picked: number
    groups: TableGroup[]
    armed: boolean
    thinking: boolean
    notice: string
    ongroup: () => void
    onsuggest?: () => void
    onarrange: () => void
    ondisarm: () => void
    ondismiss: () => void
    onshow: (id: string) => void
    ondrop: (id: string) => void
  }

  let {
    words,
    picked,
    groups,
    armed,
    thinking,
    notice,
    ongroup,
    onsuggest,
    onarrange,
    ondisarm,
    ondismiss,
    onshow,
    ondrop,
  }: Props = $props()
</script>

{#snippet swap(first: string, second: string, flipped: boolean)}
  <span class="grid">
    <span class={["col-start-1 row-start-1", flipped && "invisible"]}>
      {first}
    </span>

    <span class={["col-start-1 row-start-1", !flipped && "invisible"]}>
      {second}
    </span>
  </span>
{/snippet}

<div class="flex max-w-96 flex-col items-end gap-2">
  <div
    role="toolbar"
    aria-label={words.board}
    class="flex items-center gap-1 bg-base-100 p-1 hairline"
  >
    {#if picked > 1}
      <button
        type="button"
        onclick={ongroup}
        in:rise
        class="btn btn-ghost btn-sm font-medium"
      >
        <Icon icon="lucide:group" class="size-4" />
        {words.group}
      </button>
    {/if}

    {#if onsuggest}
      <button
        type="button"
        onclick={onsuggest}
        aria-busy={thinking}
        class="btn btn-ghost btn-sm font-medium"
      >
        <Icon
          icon={thinking ? "lucide:loader-circle" : "lucide:sparkles"}
          class={["size-4", thinking ? "animate-spin" : "text-primary"]}
        />
        {@render swap(words.think, words.cancel, thinking)}
      </button>
    {/if}

    <button
      type="button"
      onclick={onarrange}
      aria-pressed={armed}
      class={["btn btn-sm font-medium", armed ? "btn-warning" : "btn-ghost"]}
    >
      <Icon icon="lucide:wand-sparkles" class="size-4" />
      {@render swap(words.auto, words.picked, picked > 0)}
    </button>
  </div>

  {#if armed && picked === 0}
    <div
      role="status"
      in:rise
      out:leave
      class={[
        "flex max-w-80 items-center gap-2 floating py-1 pr-1 pl-3 text-xs",
        "hairline",
      ]}
    >
      <Icon icon="lucide:triangle-alert" class="size-4 shrink-0" />

      <span class="min-w-0 flex-1 text-pretty">{words.warn}</span>

      <button
        type="button"
        onclick={ondisarm}
        aria-keyshortcuts="Escape"
        class="btn btn-ghost btn-xs"
      >
        {words.cancel}
      </button>
    </div>
  {/if}

  {#if notice}
    <div
      role="status"
      in:rise
      out:leave
      class={[
        "flex max-w-80 items-center gap-2 floating py-1 pr-1 pl-3 text-xs",
        "hairline",
      ]}
    >
      <span class="min-w-0 flex-1 text-pretty text-error select-text">
        {notice}
      </span>

      <button
        type="button"
        aria-label={words.dismiss}
        onclick={ondismiss}
        class="btn btn-square btn-ghost btn-xs"
      >
        <Icon icon="lucide:x" class="size-3" />
      </button>
    </div>
  {/if}

  {#if groups.length > 0}
    <ul class="flex flex-wrap justify-end gap-1">
      {#each groups as group (group.id)}
        <li
          in:rise
          class="flex items-center gap-1 bg-base-100 pl-2 text-xs hairline"
        >
          <button
            type="button"
            onclick={() => onshow(group.id)}
            class={[
              "flex cursor-pointer items-center gap-2 py-1",
              "transition-colors hover:text-primary",
            ]}
          >
            <span aria-hidden="true" class="size-2 bg-primary"></span>
            <span class="max-w-32 truncate">{group.name}</span>
            <span class="text-base-content/70 tabular-nums">
              {group.tables.length}
            </span>
          </button>

          <button
            type="button"
            aria-label="{words.ungroup}, {group.name}"
            onclick={() => ondrop(group.id)}
            class="btn btn-square btn-ghost btn-xs"
          >
            <Icon icon="lucide:x" class="size-3" />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
