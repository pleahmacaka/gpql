<script lang="ts">
  import { calm } from "@gpql/ui/motion/index.ts"
  import SchemaBoard from "@gpql/ui/schema/SchemaBoard.svelte"
  import { board } from "@gpql/ui/schema/board.svelte.ts"
  import { toFlow } from "@gpql/ui/schema/levels.ts"
  import type { SchemaTable } from "@gpql/ui/types.ts"
  import { SvelteFlowProvider } from "@xyflow/svelte"
  import { onDestroy, untrack } from "svelte"

  import { schema } from "../sample"

  type Props = { active: boolean }

  type Step = "scattered" | "gliding" | "drawing" | "settled"

  let { active }: Props = $props()

  const GLIDE = 820
  const DRAW = 1100

  const still = calm()

  let step = $state<Step>(still ? "settled" : "scattered")
  let tables = $state.raw<SchemaTable[]>(schema)
  let timers: ReturnType<typeof setTimeout>[] = []

  function scatter() {
    const placed = toFlow(schema).nodes.filter(node => node.type === "table")

    return Object.fromEntries(
      placed.map((node, index) => {
        const swap = placed[(index + 2) % placed.length].position

        return [
          node.id,
          {
            x: swap.x + ((index * 53) % 90) - 45,
            y: swap.y + ((index * 71) % 120) - 60,
          },
        ]
      }),
    )
  }

  function arrange() {
    const later = (ms: number, next: () => void) =>
      timers.push(setTimeout(next, ms))

    later(360, () => {
      step = "gliding"
      board.spots = {}
      tables = [...schema]
    })

    later(360 + GLIDE, () => (step = "drawing"))
    later(360 + GLIDE + DRAW, () => (step = "settled"))
  }

  if (!still) {
    board.spots = scatter()
  }

  $effect(() => {
    if (active && untrack(() => step) === "scattered") {
      untrack(arrange)
    }
  })

  onDestroy(() => {
    timers.forEach(clearTimeout)
    board.reset()
  })
</script>

<div class="h-full p-2">
  <div class={["board h-full overflow-hidden bg-base-100 lift", step]}>
    <SvelteFlowProvider>
      <SchemaBoard {tables} dark keyboard={false} minimap={false} />
    </SvelteFlowProvider>
  </div>
</div>

<style>
  .scattered :global(.svelte-flow__edge),
  .gliding :global(.svelte-flow__edge) {
    opacity: 0;
  }

  .gliding :global(.svelte-flow__node) {
    transition: transform 820ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .drawing :global(.svelte-flow__edge-path) {
    stroke-dasharray: 2400;
    stroke-dashoffset: 2400;
    animation: draw 1100ms ease-out forwards;
  }

  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
</style>
