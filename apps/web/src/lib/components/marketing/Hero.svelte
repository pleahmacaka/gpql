<script lang="ts">
  import { field } from "@gpql/ui/ascii/field.ts"
  import { mark } from "@gpql/ui/ascii/mark.ts"
  import { rem } from "@gpql/ui/controls/rem.ts"
  import { Icon } from "@gpql/ui/icons/index.ts"
  import { scramble } from "@gpql/ui/motion/scramble.ts"
  import type { Attachment } from "svelte/attachments"

  import { backends } from "./sample"

  type Props = { release: string; repo: string }

  type Pointer = { x: number; y: number; width: number; height: number }

  let { release, repo }: Props = $props()

  const CELL = 0.75
  const LEFT = [0.16, 0.4, 0.64, 0.86]
  const RIGHT = [0.28, 0.52, 0.76]

  let pointer: Pointer | null = null
  let rushing = false

  function hot() {
    if (!pointer) {
      return null
    }

    const rows = Math.floor(pointer.height / rem(CELL))
    const row = Math.floor(pointer.y / rem(CELL))
    const left = pointer.x < pointer.width / 2
    const lane = (left ? LEFT : RIGHT).findIndex(
      share => Math.abs(Math.floor(share * rows) - row) <= 1,
    )

    if (lane === -1) {
      return null
    }

    return left ? lane : lane + LEFT.length
  }

  const signature = mark({
    lanes: { left: LEFT, right: RIGHT },
    hot,
    busy: () => rushing,
    start: performance.now() / 1000,
  })

  const track: Attachment<HTMLElement> = node => {
    const move = (event: PointerEvent) => {
      const box = node.getBoundingClientRect()

      pointer = {
        x: event.clientX - box.left,
        y: event.clientY - box.top,
        width: box.width,
        height: box.height,
      }
    }

    const leave = () => {
      pointer = null
    }

    node.addEventListener("pointermove", move)
    node.addEventListener("pointerleave", leave)

    return () => {
      node.removeEventListener("pointermove", move)
      node.removeEventListener("pointerleave", leave)
    }
  }

  const rush = (on: boolean) => () => {
    rushing = on
  }
</script>

<section
  aria-labelledby="hero-title"
  class="relative isolate overflow-hidden"
>
  <div
    aria-hidden="true"
    class="hud hud-wide pointer-events-none absolute inset-4 sm:inset-6"
  ></div>

  <div aria-hidden="true" class="band relative" {@attach track}>
    <div class="absolute inset-0 mask-b-from-55% mask-x-from-90%">
      <div class="grid-field absolute inset-0"></div>
      <div class="grain absolute inset-0 opacity-5"></div>
    </div>

    <canvas
      class="absolute inset-0 size-full"
      {@attach field(signature, { cell: CELL })}
    ></canvas>
  </div>

  <div class="relative mx-auto max-w-4xl px-8 pb-8 text-center">
    <h1
      id="hero-title"
      use:scramble={{ duration: 900 }}
      class="text-7xl leading-none font-extrabold tracking-tight"
    >
      GPQL
    </h1>

    <p
      class={[
        "mx-auto max-w-2xl pt-4 text-xl text-pretty text-base-content/80",
        "sm:text-2xl",
      ]}
    >
      Databases, brokers and buckets in one native window.
    </p>

    <div class="flex flex-wrap justify-center gap-2 pt-8">
      <a
        href={release}
        class="btn btn-lg btn-primary"
        onpointerenter={rush(true)}
        onpointerleave={rush(false)}
        onfocus={rush(true)}
        onblur={rush(false)}
      >
        <Icon icon="lucide:download" class="size-5" />
        Download for Windows
      </a>

      <a href={repo} class="btn btn-soft btn-lg font-normal">
        <Icon icon="simple-icons:github" class="size-5" />
        Source
      </a>
    </div>

    <ul
      class={[
        "flex flex-wrap justify-center gap-2 pt-6 text-sm",
        "text-base-content/80",
      ]}
    >
      <li class="flex items-center gap-2 px-3 py-1 hairline">
        <span class="font-semibold text-primary tabular-nums">
          {backends.length}
        </span>
        backends
      </li>

      <li class="flex items-center gap-2 px-3 py-1 hairline">
        <Icon icon="lucide:badge-check" class="size-4 text-primary" />
        Free
      </li>

      <li class="flex items-center gap-2 px-3 py-1 hairline">
        <Icon icon="lucide:user-round-x" class="size-4 text-primary" />
        No account needed
      </li>

      <li class="flex items-center gap-2 px-3 py-1 hairline">
        <Icon icon="simple-icons:windows" class="size-4 text-primary" />
        Windows 10 and 11
      </li>
    </ul>
  </div>
</section>

<style>
  .band {
    height: clamp(16rem, 40svh, 28rem);
  }
</style>
