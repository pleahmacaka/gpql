<script lang="ts">
  import { Icon } from "@gpql/ui/icons/index.ts"
  import { calm } from "@gpql/ui/motion/index.ts"
  import { scramble } from "@gpql/ui/motion/scramble.ts"
  import type { Attachment } from "svelte/attachments"

  import { backends } from "./sample"

  const DIALECTS = [
    { id: "sql", label: "SQL" },
    { id: "cypher", label: "Cypher" },
    { id: "flux", label: "Flux" },
    { id: "mqtt", label: "MQTT" },
    { id: "s3", label: "S3" },
  ]

  const NOISE = [..."abcdefghijklmnopqrstuvwxyz0123456789-_"]

  let lit = $state<string | null>(null)
  let swept = $state(false)

  const tally = (dialect: string) =>
    backends.filter(entry => entry.dialect === dialect).length

  const sweep: Attachment<HTMLElement> = node => {
    const seen = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          swept = true
          seen.disconnect()
        }
      },
      { threshold: 0.35 },
    )

    seen.observe(node)

    return () => seen.disconnect()
  }

  const reshuffle: Attachment<HTMLElement> = node => {
    const target = node.textContent ?? ""
    const tile = node.closest("li")
    let frame = 0

    const run = () => {
      if (calm()) {
        return
      }

      cancelAnimationFrame(frame)

      const started = performance.now()

      const step = (now: number) => {
        const progress = Math.min(1, (now - started) / 380)
        const locked = progress * target.length

        node.textContent = [...target]
          .map((char, index) =>
            char === " " || index < locked
              ? char
              : NOISE[Math.floor(Math.random() * NOISE.length)],
          )
          .join("")

        if (progress < 1) {
          frame = requestAnimationFrame(step)
        }
      }

      frame = requestAnimationFrame(step)
    }

    tile?.addEventListener("pointerenter", run)

    return () => {
      cancelAnimationFrame(frame)
      tile?.removeEventListener("pointerenter", run)
      node.textContent = target
    }
  }
</script>

<section
  id="backends"
  aria-labelledby="backends-title"
  class="border-t border-base-content/10"
>
  <div class="mx-auto max-w-6xl px-4 py-24 sm:px-6">
    <div class="flex flex-wrap items-end justify-between gap-8">
      <div>
        <p class="flex items-center gap-3 text-sm text-primary">
          <span class="size-2 bg-primary"></span>
          Backends
        </p>

        <h2
          id="backends-title"
          class={[
            "pt-4 text-5xl font-extrabold tracking-tight text-balance",
            "sm:text-7xl",
          ]}
        >
          <span class="text-primary tabular-nums">{backends.length}</span>
          <span use:scramble={{ onView: true }}>backends.</span>
        </h2>

        <p class="pt-4 text-lg text-base-content/80">
          Each one through its own driver.
        </p>
      </div>

      <div
        role="group"
        aria-label="Light up by query language"
        class="flex flex-wrap gap-2"
      >
        {#each DIALECTS as dialect (dialect.id)}
          <button
            type="button"
            aria-pressed={lit === dialect.id}
            onclick={() => (lit = lit === dialect.id ? null : dialect.id)}
            class={[
              "btn font-normal btn-sm",
              lit === dialect.id ? "btn-primary" : "btn-soft",
            ]}
          >
            {dialect.label}
            <span class="tabular-nums opacity-70">{tally(dialect.id)}</span>
          </button>
        {/each}
      </div>
    </div>

    <ul
      class="grid grid-cols-2 gap-2 pt-12 sm:grid-cols-3 lg:grid-cols-4"
      {@attach sweep}
    >
      {#each backends as backend, index (backend.id)}
        <li
          style:animation-delay="{index * 55}ms"
          class={[
            "hud flex flex-col gap-6 bg-base-100 p-5 hover:hud-lit",
            lit === backend.dialect && "hud-lit",
            swept && "swept",
          ]}
        >
          <div class="flex items-start justify-between gap-2">
            <Icon
              icon={backend.icon}
              class={[
                "size-8 transition-colors",
                lit === backend.dialect && "text-primary",
              ]}
            />

            {#if backend.wip}
              <span class="badge badge-ghost badge-sm">WIP</span>
            {/if}
          </div>

          <div class="min-w-0">
            <p class="truncate text-base font-semibold">{backend.label}</p>
            <p
              class="truncate text-sm text-base-content/70"
              {@attach reshuffle}
            >
              {backend.driver}
            </p>
          </div>
        </li>
      {/each}
    </ul>

    <p class="pt-6 text-sm text-pretty text-base-content/70">
      WIP: GPQL doesn't read column lists or sort and page on the server there
      yet.
    </p>
  </div>
</section>

<style>
  .swept {
    animation: sweep 1100ms ease-out backwards;
  }

  @keyframes sweep {
    from {
      --hud-on: 0;
    }

    35% {
      --hud-on: 1;
    }

    to {
      --hud-on: 0;
    }
  }
</style>
