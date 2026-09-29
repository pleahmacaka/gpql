<script lang="ts">
  import { Icon } from "@gpql/ui/icons/index.ts"
  import { calm } from "@gpql/ui/motion/index.ts"
  import { scramble } from "@gpql/ui/motion/scramble.ts"
  import { type Component, untrack } from "svelte"
  import type { Attachment } from "svelte/attachments"

  import AppWindow from "./AppWindow.svelte"
  import { CHAPTERS, type Chapter } from "./chapters"
  import ConnectView from "./views/ConnectView.svelte"

  type View = Component<{ active: boolean; onnext?: () => void }>

  let active = $state(0)
  let seen = $state(false)
  let reach = $state(0)

  let views = $state.raw<Partial<Record<Chapter["id"], View>>>({
    connect: ConnectView,
  })

  let steps: HTMLElement[] = []

  let chapter = $derived(CHAPTERS[active])

  $effect(() => {
    if (seen && active + 1 > untrack(() => reach)) {
      reach = active + 1
    }
  })

  async function fetchViews() {
    const [data, query, schema] = await Promise.all([
      import("./views/DataView.svelte"),
      import("./views/QueryView.svelte"),
      import("./views/SchemaView.svelte"),
    ])

    views = {
      ...views,
      data: data.default,
      query: query.default,
      schema: schema.default,
    }
  }

  const prefetch: Attachment<HTMLElement> = node => {
    const near = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          near.disconnect()
          fetchViews()
        }
      },
      { rootMargin: "100% 0%" },
    )

    near.observe(node)

    return () => near.disconnect()
  }

  const follow: Attachment<HTMLElement> = node => {
    steps = [...node.children].filter(child => child instanceof HTMLElement)

    const watch = new IntersectionObserver(
      entries => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const index = steps.findIndex(step => step === entry.target)

            active = Math.min(index, CHAPTERS.length - 1)
          }
        }
      },
      { rootMargin: "-25% 0% -74% 0%" },
    )

    for (const step of steps) {
      watch.observe(step)
    }

    return () => watch.disconnect()
  }

  const spot: Attachment<HTMLElement> = node => {
    const watch = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          seen = true
          watch.disconnect()
        }
      },
      { rootMargin: "0% 0% -12% 0%" },
    )

    watch.observe(node)

    return () => watch.disconnect()
  }

  function go(index: number) {
    const step = steps[index]

    if (step?.checkVisibility()) {
      step.scrollIntoView({
        behavior: calm() ? "instant" : "smooth",
        block: "start",
      })

      return
    }

    active = index
  }

  const tabbed = (tab: string) =>
    go(CHAPTERS.findIndex(entry => entry.id === tab))
</script>

<section
  id="tour"
  aria-labelledby="tour-title"
  class="tour relative"
  {@attach prefetch}
>
  <h2 id="tour-title" class="sr-only">A tour of GPQL</h2>

  <div class="grid grid-cols-1">
    <div
      aria-hidden="true"
      class={[
        "pointer-events-none col-start-1 row-start-1 hidden",
        "lg:motion-safe:block",
      ]}
      {@attach follow}
    >
      {#each CHAPTERS as entry (entry.id)}
        <div class="h-svh"></div>
      {/each}

      <div class="tail"></div>
    </div>

    <div
      class={[
        "pin col-start-1 row-start-1 flex flex-col gap-4 self-start",
        "px-4 pt-4 pb-16 sm:px-6",
      ]}
    >
      <div
        class={[
          "pane relative mx-auto flex w-full max-w-7xl flex-wrap items-center",
          "gap-x-6 gap-y-2 px-2 py-2",
        ]}
      >
        <ol aria-label="Tour" class="flex max-w-full gap-1 overflow-x-auto">
          {#each CHAPTERS as entry, index (entry.id)}
            <li>
              <button
                type="button"
                aria-current={index === active ? "step" : undefined}
                onclick={() => go(index)}
                class={[
                  "relative flex cursor-pointer items-center gap-2 px-3 py-2",
                  "text-sm transition-colors",
                  index === active
                    ? "text-base-content"
                    : "text-base-content/70 hover:text-base-content",
                ]}
              >
                <span class="text-xs text-primary tabular-nums">
                  0{index + 1}
                </span>
                {entry.title}
                <span
                  aria-hidden="true"
                  class={[
                    "absolute inset-x-3 bottom-0 h-0 origin-left border-b-2",
                    "border-primary transition-transform",
                    index === active ? "scale-x-100" : "scale-x-0",
                  ]}
                ></span>
              </button>
            </li>
          {/each}
        </ol>

        <div class="min-w-64 flex-1 px-3 lg:px-0">
          {#key chapter.id}
            <p
              class="truncate text-base font-semibold"
              use:scramble={{ duration: 520 }}
            >
              {chapter.line}
            </p>
          {/key}

          <ul
            aria-label="In {chapter.title}"
            class="flex gap-2 overflow-hidden pt-1"
          >
            {#each chapter.chips as chip (chip)}
              <li
                class={[
                  "shrink-0 px-2 py-1 text-xs whitespace-nowrap",
                  "text-base-content/70 hairline",
                ]}
              >
                {chip}
              </li>
            {/each}
          </ul>
        </div>

        <p
          class={[
            "hidden w-56 items-center justify-end gap-2 pr-3 text-sm",
            "text-base-content/70 xl:flex",
          ]}
        >
          <Icon
            icon="lucide:mouse-pointer-click"
            class="size-4 shrink-0 text-primary"
          />
          {chapter.hint}
        </p>

        <span
          aria-hidden="true"
          class={[
            "progress absolute inset-x-0 bottom-0 h-0",
            "border-b border-primary",
          ]}
        ></span>
      </div>

      <div
        class={[
          "rise mx-auto h-144 w-full max-w-7xl",
          "lg:motion-safe:h-auto lg:motion-safe:min-h-0 lg:motion-safe:flex-1",
        ]}
        {@attach spot}
      >
        <AppWindow
          chip={chapter.tab ? "roomy" : "no session"}
          chipIcon={chapter.tab ? "simple-icons:postgresql" : "lucide:plus"}
          tone={chapter.tab ? "live" : "idle"}
          tab={chapter.tab}
          onchip={() => go(0)}
          ontab={tabbed}
        >
          {#each CHAPTERS as entry, index (entry.id)}
            {@const View = views[entry.id]}

            <div
              inert={index !== active}
              class={[
                "view col-start-1 row-start-1 min-h-0",
                index < active && "before",
                index > active && "after",
              ]}
            >
              {#if View && index <= reach}
                <View active={seen && index === active} onnext={() => go(1)} />
              {/if}
            </div>
          {/each}
        </AppWindow>
      </div>
    </div>
  </div>
</section>

<style>
  .view {
    transition:
      opacity 280ms ease-out,
      translate 280ms cubic-bezier(0.2, 0.8, 0.2, 1),
      visibility 280ms;
  }

  .before,
  .after {
    opacity: 0;
    visibility: hidden;
  }

  .before {
    translate: -2rem 0;
  }

  .after {
    translate: 2rem 0;
  }

  .tail {
    height: 50svh;
  }

  .progress {
    display: none;
  }

  @media (min-width: 64rem) and (prefers-reduced-motion: no-preference) {
    .pin {
      position: sticky;
      top: 4rem;
      height: calc(100svh - 4rem);
      padding-bottom: 1.5rem;
    }

    @supports (animation-timeline: view()) {
      .tour {
        view-timeline-name: --tour;
      }

      .progress {
        display: block;
        transform-origin: left;
        animation: fill linear both;
        animation-timeline: --tour;
        animation-range: contain 0% contain 100%;
      }

      .rise {
        animation: rise linear both;
        animation-timeline: view();
        animation-range: entry 0% entry 100%;
      }
    }
  }

  @keyframes fill {
    from {
      transform: scaleX(0);
    }
  }

  @keyframes rise {
    from {
      transform: perspective(160rem) rotateX(16deg) scale(0.92);
    }
  }
</style>
