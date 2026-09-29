<script lang="ts">
  import type { Snippet } from "svelte"

  import { field, type Scene } from "../ascii/field"
  import { mark } from "../ascii/mark"
  import { graph, link, sheet } from "../ascii/scenes"

  type Art = "mark" | "sheet" | "graph" | "link"

  type Props = {
    title: string
    hint?: string
    art?: Art | null
    class?: string
    children?: Snippet
  }

  let {
    title,
    hint = "",
    art = "mark",
    class: extra = "",
    children,
  }: Props = $props()

  const SCENES: Record<Art, () => Scene> = {
    mark: () => mark({ start: performance.now() / 1000 }),
    sheet,
    graph,
    link,
  }

  let scene = $derived(art ? SCENES[art]() : null)
</script>

<div
  class={[
    "flex flex-col items-center justify-center gap-4 p-8 text-center",
    extra,
  ]}
>
  {#if scene}
    <canvas
      aria-hidden="true"
      class="h-32 w-80 max-w-full"
      {@attach field(scene, { cell: 0.625, fps: 20 })}
    ></canvas>
  {/if}

  <div class="flex max-w-sm flex-col items-center gap-1">
    <p class="text-sm font-medium text-balance">{title}</p>

    {#if hint}
      <p class="text-xs text-pretty text-base-content/70">{hint}</p>
    {/if}
  </div>

  {#if children}
    <div class="flex flex-wrap items-center justify-center gap-2">
      {@render children()}
    </div>
  {/if}
</div>
