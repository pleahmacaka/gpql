<script lang="ts">
  import { field, type Scene } from "@gpql/ui/ascii/field.ts"
  import { mark } from "@gpql/ui/ascii/mark.ts"
  import type { Attachment } from "svelte/attachments"

  type Props = { busy?: () => boolean }

  let { busy }: Props = $props()

  let scene = $state<Scene | null>(null)

  const watch: Attachment<HTMLElement> = node => {
    const seen = new IntersectionObserver(
      ([entry]) => {
        if (!entry.isIntersecting) {
          return
        }

        seen.disconnect()
        scene = mark({ start: performance.now() / 1000, busy })
      },
      { rootMargin: "0% 0% -10% 0%" },
    )

    seen.observe(node)

    return () => seen.disconnect()
  }
</script>

<div aria-hidden="true" class="relative size-72 sm:size-96" {@attach watch}>
  {#if scene}
    <canvas
      class="absolute inset-0 size-full"
      {@attach field(scene, { cell: 0.5 })}
    ></canvas>
  {/if}
</div>
