<script lang="ts">
  import Segmented from "@gpql/ui/controls/Segmented.svelte"
  import DataGrid from "@gpql/ui/data/DataGrid.svelte"
  import ResultChart from "@gpql/ui/data/ResultChart.svelte"
  import { Icon } from "@gpql/ui/icons/index.ts"
  import { calm } from "@gpql/ui/motion/index.ts"
  import { onDestroy, untrack } from "svelte"

  import { busiest, saved } from "../sample"
  import Sql from "../Sql.svelte"

  type Props = { active: boolean }

  let { active }: Props = $props()

  const PROMPT = busiest.prompt
  const SQL = busiest.sql
  const TOTAL = busiest.rows.length

  const LETTER = 34
  const STROKE = 5
  const ROW = 80

  const WRITE = PROMPT.length * LETTER + 260
  const RUN = WRITE + SQL.length * STROKE + 240
  const FILL = RUN + 560
  const END = FILL + TOTAL * ROW
  const FLIP = END + 1200

  let clock = $state(-1)
  let view = $state("table")
  let touched = false
  let frame = 0

  const within = (value: number, top: number) =>
    Math.max(0, Math.min(top, Math.floor(value)))

  let asked = $derived(within(clock / LETTER, PROMPT.length))
  let written = $derived(within((clock - WRITE) / STROKE, SQL.length))
  let running = $derived(clock >= RUN && clock < FILL)
  let shown = $derived(
    clock < FILL ? 0 : within((clock - FILL) / ROW + 1, TOTAL),
  )
  let rows = $derived(busiest.rows.slice(0, shown))

  let status = $derived.by(() => {
    if (running) {
      return "running"
    }

    if (clock >= END) {
      return `${TOTAL} rows`
    }

    return ""
  })

  function play(from: number) {
    cancelAnimationFrame(frame)

    if (calm()) {
      clock = END

      return
    }

    const origin = performance.now() - from
    const flips = from === 0

    const step = (now: number) => {
      clock = now - origin

      if (clock < FLIP) {
        frame = requestAnimationFrame(step)

        return
      }

      if (flips && !touched) {
        view = "chart"
      }
    }

    frame = requestAnimationFrame(step)
  }

  $effect(() => {
    if (active && untrack(() => clock) < 0) {
      play(0)
    }
  })

  onDestroy(() => cancelAnimationFrame(frame))
</script>

<div class="flex h-full gap-2 p-2">
  <nav
    aria-label="Saved queries"
    class={[
      "hidden w-48 shrink-0 flex-col gap-1",
      "rounded-box bg-base-100 p-2 lift xl:flex",
    ]}
  >
    <p class="px-2 py-1 text-xs text-base-content/70">Saved</p>

    {#each saved as name, index (name)}
      <p
        class={[
          "truncate rounded-field px-2 py-2 text-sm",
          index === 0 && "bg-primary/10 text-primary",
        ]}
      >
        {name}
      </p>
    {/each}
  </nav>

  <section
    aria-label="Query"
    class={[
      "flex min-w-0 flex-1 flex-col gap-2",
      "rounded-box bg-base-100 p-2 lift",
    ]}
  >
    <header class="flex items-center gap-2 pl-2">
      <h3 class="text-sm font-medium">Query</h3>

      <span
        class={[
          "flex flex-1 items-center gap-2 truncate text-xs tabular-nums",
          "text-base-content/70",
        ]}
      >
        {#if running}
          <Icon
            icon="lucide:loader-circle"
            class="size-4 animate-spin text-primary"
          />
        {/if}
        {status}
      </span>

      <button
        type="button"
        class="btn btn-sm btn-primary"
        disabled={running}
        onclick={() => play(RUN)}
      >
        <Icon icon="lucide:play" class="size-4" />
        Run
      </button>

      <div class="w-44">
        <Segmented
          bind:value={view}
          onpick={() => (touched = true)}
          options={[
            { value: "table", label: "Table" },
            { value: "chart", label: "Graph" },
          ]}
        />
      </div>
    </header>

    <p
      class={[
        "flex items-center gap-2 rounded-field",
        "bg-base-200 px-4 py-2 text-sm",
      ]}
    >
      <Icon icon="lucide:sparkles" class="size-4 shrink-0 text-accent" />
      <span class="grid">
        <span aria-hidden="true" class="invisible col-start-1 row-start-1">
          {PROMPT}
        </span>
        <span class="col-start-1 row-start-1">
          {PROMPT.slice(0, asked)}
        </span>
      </span>
    </p>

    <div class="grid">
      <div aria-hidden="true" class="invisible col-start-1 row-start-1">
        <Sql code={SQL} />
      </div>

      <div class="col-start-1 row-start-1 min-w-0">
        <Sql
          code={SQL.slice(0, written)}
          caret={clock >= WRITE && clock < RUN}
        />
      </div>
    </div>

    <div class="flex min-h-0 flex-1 flex-col pt-2">
      {#if view === "chart"}
        <ResultChart columns={busiest.columns} rows={busiest.rows} />
      {:else}
        <DataGrid
          columns={busiest.columns}
          {rows}
          busy={running}
          types={{ topic: "text", messages: "bigint" }}
          minimap={false}
          wheelPan={false}
        />
      {/if}
    </div>
  </section>
</div>
