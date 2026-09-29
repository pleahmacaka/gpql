<script lang="ts">
  import { eq } from "drizzle-orm"
  import { onMount, tick } from "svelte"
  import { fade } from "svelte/transition"

  import {
    arrive,
    drag,
    EmptyState,
    field,
    Icon,
    Keycap,
    Lazy,
    Marker,
    mark,
    Panel,
    rem,
    Segmented,
    veil,
  } from "@gpql/ui"

  import ResultGrid from "$lib/components/data/ResultGrid.svelte"
  import TransactionBar from "$lib/components/data/TransactionBar.svelte"
  import FindBar from "$lib/components/shell/FindBar.svelte"
  import TabLayout from "$lib/components/shell/TabLayout.svelte"
  import { local } from "$lib/db/client"
  import { preference } from "$lib/db/schema"
  import * as m from "$lib/paraglide/messages"
  import { hint } from "$lib/session/errors"
  import { exportResult, FORMATS } from "$lib/session/exporting"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { ExportFormat } from "$lib/types"

  import { menuBelow } from "./anchor"
  import AskDialog from "./AskDialog.svelte"
  import { chartLabels } from "./chart"
  import { offerFormat } from "./format"
  import PlanPanel from "./PlanPanel.svelte"
  import QueryBuilder from "./QueryBuilder.svelte"
  import SavedQueries from "./SavedQueries.svelte"
  import SqlEditor from "./SqlEditor.svelte"

  type Mode = "builder" | "script"

  const CHROME = 13.25
  const SPLIT: Record<Mode, number> = { script: 13, builder: 20 }

  let asking = $state(false)
  let building = $state(false)
  let built = $state("")
  let charting = $state(false)
  let term = $state("")
  let hit = $state(0)
  let editor = $state<SqlEditor | null>(null)
  let stopping = $state(false)
  let split = $state({ ...SPLIT })
  let room = $state(0)
  let well = $state(0)
  let slow = $state(false)
  let elapsed = $state(0)

  let query = $derived(workspace.query)
  let mode = $derived<Mode>(building ? "builder" : "script")
  let picked = $derived(query.selection.end > query.selection.start)
  let advice = $derived(hint(query.error ?? ""))
  let composeAdvice = $derived(hint(query.composeError))
  let stopped = $derived(query.error === m.query_stopped())

  let title = $derived(
    query.saved.find(entry => entry.id === query.open)?.name ??
      m.query_untitled(),
  )

  // mqtt has no statement to build; an empty builder would clear its buffer
  let buildable = $derived(
    !!workspace.session?.sliceable && workspace.session.kind !== "mqtt",
  )

  let explainable = $derived(workspace.active?.canExplain ?? false)
  let analyzable = $derived(workspace.active?.canAnalyze ?? false)

  let ready = $derived(building ? built !== "" : query.chosen !== "")

  let rows = $derived(query.result?.rows.length ?? 0)

  let phase = $derived.by(() => {
    if (query.busy && slow) {
      return "busy"
    }

    if (query.fault) {
      return "fault"
    }

    if (query.error) {
      return "error"
    }

    if (query.composeError) {
      return "compose"
    }

    if (!query.result) {
      return "empty"
    }

    if (query.result.columns.length === 0) {
      return "touched"
    }

    return charting && rows > 0 ? "chart" : "grid"
  })

  let height = $derived(
    room === 0
      ? split[mode]
      : Math.min(split[mode], Math.max(6, room / rem(1) - CHROME)),
  )

  let took = $derived(
    query.busy || query.error || query.fault ? null : query.millis,
  )

  let hits = $derived.by(() => {
    const needle = term.trim().toLowerCase()

    if (needle === "") {
      return []
    }

    const hay = query.sql.toLowerCase()
    const out: number[] = []
    let at = hay.indexOf(needle)

    while (at !== -1) {
      out.push(at)
      at = hay.indexOf(needle, at + needle.length)
    }

    return out
  })

  onMount(async () => {
    const stored = await local
      .select()
      .from(preference)
      .where(eq(preference.key, "querySplit"))

    const [script, builder] = (stored[0]?.value ?? "").split(",").map(Number)

    split = {
      script: script || SPLIT.script,
      builder: builder || SPLIT.builder,
    }
  })

  onMount(() => offerFormat(formatSql))

  // reveal() on every keystroke would pull the focus out of the find box
  $effect(() => {
    void term
    hit = 0
  })

  $effect(() => {
    const text = query.sql
    const id = query.open
    const stored = query.saved.find(entry => entry.id === id)?.sql

    if (!id || text.trim() === "" || text === stored) {
      return
    }

    query.autosaved = false

    const timer = setTimeout(async () => {
      await query.keep()
      query.autosaved = true
    }, 1200)

    return () => clearTimeout(timer)
  })

  $effect(() => {
    if (!query.busy) {
      return
    }

    const started = performance.now()

    elapsed = 0
    slow = false

    const reveal = setTimeout(() => (slow = true), 240)
    const ticker = setInterval(
      () => (elapsed = performance.now() - started),
      1000,
    )

    return () => {
      clearTimeout(reveal)
      clearInterval(ticker)
      slow = false
    }
  })

  $effect(() => {
    if (!buildable) {
      building = false
    }
  })

  $effect(() => {
    if (workspace.dialect === "flux") {
      building = true
    }
  })

  $effect(() => {
    if (workspace.finding && workspace.tab === "query") {
      building = false
    }
  })

  function show() {
    const at = hits[Math.min(hit, hits.length - 1)]

    if (at !== undefined) {
      editor?.reveal(at, at + term.trim().length)
    }
  }

  function step(by: number) {
    if (hits.length > 0) {
      hit = (hit + by + hits.length) % hits.length
      show()
    }
  }

  function doneFinding() {
    workspace.finding = false
    query.selection = { start: 0, end: 0 }
  }

  function pickMode(next: string) {
    building = next === "builder"

    if (!building) {
      void tick().then(() => editor?.focus())
    }
  }

  async function formatSql() {
    building = false
    await tick()
    await editor?.format()
  }

  async function adopt() {
    if (built !== "") {
      await query.replace(built)
    }
  }

  async function runNow() {
    query.spot = false
    query.composeError = ""

    if (building) {
      await adopt()
    }

    await query.run()
  }

  async function explain(analyze: boolean) {
    if (building) {
      await adopt()
    }

    await query.explain(analyze)
  }

  async function toEditor() {
    await adopt()
    pickMode("script")
  }

  async function stop() {
    stopping = true

    try {
      await query.stop()
    } finally {
      stopping = false
    }
  }

  async function shipOut(format: ExportFormat) {
    const session = workspace.session
    const result = query.result

    if (!session || !result) {
      return
    }

    try {
      workspace.notice = (await exportResult(session.id, result, format)) ?? ""
    } catch (failure) {
      workspace.notice = m.export_failed({ reason: String(failure) })
    }
  }

  function keep() {
    void workspace.remember(
      "querySplit",
      `${split.script},${split.builder}`,
    )
  }

  function resize(event: PointerEvent) {
    const from = event.clientY
    const start = height

    drag(
      event,
      moved => {
        split[mode] = Math.max(6, start + (moved.clientY - from) / rem(1))
      },
      keep,
    )
  }

  function nudge(event: KeyboardEvent) {
    const moves: Record<string, number> = {
      ArrowUp: -1.5,
      ArrowDown: 1.5,
      PageUp: -6,
      PageDown: 6,
    }

    if (event.key === "Home" || event.key === "End") {
      event.preventDefault()
      split[mode] = event.key === "Home" ? 6 : room / rem(1) - CHROME
      keep()

      return
    }

    if (event.key in moves) {
      event.preventDefault()
      split[mode] = Math.max(6, height + moves[event.key])
      keep()
    }
  }

  function clock(millis: number) {
    return millis < 1000
      ? `${Math.round(millis)} ms`
      : `${(millis / 1000).toFixed(1)} s`
  }
</script>

{#snippet finder()}
  <FindBar
    placeholder={m.find_sql()}
    bind:term
    index={hit}
    total={hits.length}
    onnext={() => step(1)}
    onprev={() => step(-1)}
    onclose={doneFinding}
  />
{/snippet}

<TabLayout>
  {#snippet aside()}
    <SavedQueries />
  {/snippet}

  <Panel glass={false} class="min-w-0 flex-1" inner="@container">
    <div bind:clientHeight={room} class="flex min-h-0 flex-1 flex-col">
      <header
        class={[
          "flex h-11 shrink-0 items-center gap-2 border-b",
          "border-base-content/10 px-2",
        ]}
      >
        {#if buildable}
          <Segmented
            small
            label={m.query_mode()}
            value={mode}
            onpick={pickMode}
            options={[
              {
                value: "builder",
                label: m.mode_builder(),
                icon: "lucide:blocks",
              },
              { value: "script", label: m.mode_script(), icon: "lucide:code" },
            ]}
          />
        {/if}

        <h2 class="min-w-0 truncate px-2 text-sm font-medium">{title}</h2>

        {#if query.autosaved && query.open}
          <span
            transition:fade={veil()}
            class="hidden shrink-0 text-xs text-base-content/70 @2xl:inline"
          >
            {m.autosaved()}
          </span>
        {/if}

        <span class="flex-1"></span>

        {#if workspace.ai && workspace.providers.length > 0}
          <button
            type="button"
            aria-label={m.ai_write()}
            onclick={() => (asking = true)}
            disabled={query.busy}
            class="btn btn-ghost btn-sm font-medium"
          >
            <Icon icon="lucide:sparkles" class="size-4 text-primary" />
            <span class="hidden @3xl:inline">{m.ai_write()}</span>
          </button>
        {/if}

        {#if explainable && analyzable}
          <button
            type="button"
            aria-haspopup="menu"
            aria-label={m.menu_explain()}
            disabled={query.busy || !ready}
            onclick={event =>
              menuBelow(event, [
                {
                  label: m.menu_explain(),
                  icon: "lucide:git-fork",
                  run: () => explain(false),
                },
                {
                  label: m.menu_explain_analyze(),
                  icon: "lucide:timer",
                  run: () => explain(true),
                },
              ])}
            class="btn btn-ghost btn-sm font-medium"
          >
            <Icon icon="lucide:git-fork" class="size-4" />
            <span class="hidden @3xl:inline">{m.menu_explain()}</span>
            <Icon
              icon="lucide:chevron-down"
              class="size-4 text-base-content/60"
            />
          </button>
        {:else if explainable}
          <button
            type="button"
            aria-label={m.menu_explain()}
            disabled={query.busy || !ready}
            onclick={() => explain(false)}
            class="btn btn-ghost btn-sm font-medium"
          >
            <Icon icon="lucide:git-fork" class="size-4" />
            <span class="hidden @3xl:inline">{m.menu_explain()}</span>
          </button>
        {/if}

        <span class="relative grid shrink-0">
          {#if query.spot && !query.busy}
            <span
              aria-hidden="true"
              transition:fade={veil()}
              class="hud hud-lit pointer-events-none absolute -inset-1"
            ></span>
          {/if}

          <button
            type="button"
            onclick={runNow}
            disabled={query.busy || !ready}
            aria-keyshortcuts="Control+Enter"
            class={[
              "btn btn-primary btn-sm col-start-1 row-start-1 font-medium",
              (query.busy || stopping) && "invisible",
            ]}
          >
            <Icon icon="lucide:play" class="size-4" />

            <span class="grid text-left">
              <span
                class={[
                  "col-start-1 row-start-1",
                  (!picked || building) && "invisible",
                ]}
              >
                {m.run_picked()}
              </span>

              <span
                class={[
                  "col-start-1 row-start-1",
                  picked && !building && "invisible",
                ]}
              >
                {m.run_all()}
              </span>
            </span>

            <span aria-hidden="true" class="hidden @4xl:flex">
              <Keycap keys={["ctrl", "enter"]} />
            </span>
          </button>

          <button
            type="button"
            onclick={stop}
            disabled={stopping}
            aria-hidden={!(query.busy || stopping)}
            tabindex={query.busy || stopping ? 0 : -1}
            class={[
              "btn btn-soft btn-error btn-sm col-start-1 row-start-1",
              "font-medium",
              !(query.busy || stopping) && "invisible",
            ]}
          >
            <Icon
              icon={stopping ? "lucide:loader-circle" : "lucide:square"}
              class={["size-4", stopping && "animate-spin"]}
            />
            {stopping ? m.query_stopping() : m.query_stop()}
          </button>
        </span>
      </header>

      <div
        class="relative shrink-0 overflow-hidden"
        style:height="{height}rem"
      >
        {#if building}
          <div
            in:arrive={{ from: "left", distance: 1 }}
            class="absolute inset-0"
          >
            <QueryBuilder bind:built onuse={toEditor} />
          </div>
        {/if}

        <div
          class={[
            "h-full transition-opacity duration-200 starting:opacity-0",
            building && "hidden",
          ]}
        >
          <SqlEditor
            bind:this={editor}
            bind:value={query.sql}
            bind:selection={query.selection}
            onrun={runNow}
            onclear={() => query.clear()}
            find={workspace.finding ? finder : undefined}
          />
        </div>
      </div>

      <button
        type="button"
        aria-label={m.editor_resize()}
        aria-keyshortcuts="ArrowUp ArrowDown"
        onpointerdown={resize}
        onkeydown={nudge}
        class={[
          "group relative z-10 -my-1 h-2 shrink-0 cursor-row-resize",
          "outline-none",
        ]}
      >
        <span
          aria-hidden="true"
          class={[
            "absolute inset-x-0 top-1 h-0 border-t border-base-content/10",
            "transition-colors group-hover:border-primary",
            "group-focus-visible:border-t-2 group-focus-visible:border-primary",
          ]}
        ></span>
      </button>

      {#if query.plan && !(query.busy && slow)}
        <div in:fade={veil()} class="flex min-h-0 flex-1 flex-col">
          <PlanPanel />
        </div>
      {:else}
        <div
          class={[
            "flex h-10 shrink-0 items-center gap-3 border-b",
            "border-base-content/10 px-3",
          ]}
        >
          <span
            role="status"
            class="flex min-w-0 shrink-0 items-center gap-3 tabular-nums"
          >
            {#if query.busy}
              <Marker label={m.query_running()} tone="primary" />

              {#if elapsed >= 1000}
                <span class="text-xs text-base-content/70">
                  {Math.floor(elapsed / 1000)} s
                </span>
              {/if}
            {:else if query.fault}
              <Marker label={m.query_fault()} tone="warning" />
            {:else if query.error && !stopped}
              <Marker label={m.query_failed()} tone="error" />
            {:else if query.composeError}
              <Marker label={m.compose_failed()} tone="error" />
            {:else if query.result && query.result.columns.length > 0}
              <Marker label={m.rows_count({ count: rows })} tone="success" />
            {:else if query.result}
              <Marker
                label={m.rows_touched({ count: query.result.affected ?? 0 })}
                tone="success"
              />
            {:else}
              <Marker label={m.results_label()} tone="muted" />
            {/if}

            {#if took !== null}
              <span class="text-xs text-base-content/70">{clock(took)}</span>
            {/if}
          </span>

          <TransactionBar />

          <span class="flex-1"></span>

          {#if query.result && rows > 0 && !query.error}
            <Segmented
              small
              label={m.results_view()}
              value={charting ? "chart" : "table"}
              onpick={next => (charting = next === "chart")}
              options={[
                {
                  value: "table",
                  label: m.view_table(),
                  icon: "lucide:table-2",
                },
                {
                  value: "chart",
                  label: m.view_chart(),
                  icon: "lucide:chart-line",
                },
              ]}
            />
          {/if}

          {#if query.result && query.result.columns.length > 0 && !query.error}
            <button
              type="button"
              aria-haspopup="menu"
              aria-label={m.menu_export()}
              onclick={event =>
                menuBelow(
                  event,
                  FORMATS.map(format => ({
                    label: m.menu_export_as({ format: format.toUpperCase() }),
                    icon: "lucide:download",
                    run: () => shipOut(format),
                  })),
                )}
              class="btn btn-ghost btn-sm font-medium"
            >
              <Icon icon="lucide:download" class="size-4" />
              <span class="hidden @3xl:inline">{m.menu_export()}</span>
            </button>
          {/if}
        </div>

        <div
          bind:clientHeight={well}
          class="relative min-h-0 flex-1 overflow-hidden"
        >
          {#key phase}
            <div
              in:fade={veil()}
              class="absolute inset-0 flex flex-col overflow-y-auto"
            >
              {#if phase === "busy"}
                <div
                  role="status"
                  class="flex flex-1 flex-col items-center justify-center gap-4"
                >
                  <canvas
                    aria-hidden="true"
                    class="h-24 w-56 max-w-full"
                    {@attach field(
                      mark({ busy: () => true }),
                      { cell: 0.5, fps: 20 },
                    )}
                  ></canvas>

                  <p class="text-sm text-base-content/70">
                    {m.query_running()}
                  </p>
                </div>
              {:else if phase === "fault" && query.fault}
                <div
                  role="alert"
                  class="relative m-3 flex gap-3 bg-warning/5 p-4"
                >
                  <span
                    aria-hidden="true"
                    class={[
                      "hud hud-small hud-lit hud-warning pointer-events-none",
                      "absolute inset-0",
                    ]}
                  ></span>

                  <Icon
                    icon="lucide:triangle-alert"
                    class="mt-1 size-4 shrink-0 text-warning"
                  />

                  <p
                    class={[
                      "min-w-0 flex-1 text-sm leading-6 wrap-anywhere",
                      "select-text",
                    ]}
                  >
                    {m.fault_near({
                      line: query.fault.line,
                      column: query.fault.column,
                      text: query.fault.text,
                    })}
                  </p>

                  <button
                    type="button"
                    onclick={runNow}
                    disabled={query.busy}
                    class={[
                      "btn btn-soft btn-warning btn-sm shrink-0",
                      "font-medium",
                    ]}
                  >
                    <Icon icon="lucide:play" class="size-4" />
                    {m.run_anyway()}
                  </button>
                </div>
              {:else if phase === "compose"}
                <div
                  role="alert"
                  class="relative m-3 flex gap-3 bg-error/5 p-4"
                >
                  <span
                    aria-hidden="true"
                    class={[
                      "hud hud-small hud-lit hud-error pointer-events-none",
                      "absolute inset-0",
                    ]}
                  ></span>

                  <Icon
                    icon="lucide:sparkles"
                    class="mt-1 size-4 shrink-0 text-error"
                  />

                  <div class="flex min-w-0 flex-1 flex-col gap-2">
                    <p
                      class={[
                        "text-sm leading-6 whitespace-pre-wrap wrap-anywhere",
                        "select-text",
                      ]}
                    >
                      {query.composeError}
                    </p>

                    {#if composeAdvice}
                      <p class="text-xs text-base-content/70">
                        {composeAdvice}
                      </p>
                    {/if}
                  </div>

                  <button
                    type="button"
                    aria-label={m.close()}
                    onclick={() => (query.composeError = "")}
                    class="btn btn-square btn-ghost btn-sm"
                  >
                    <Icon icon="lucide:x" class="size-4" />
                  </button>
                </div>
              {:else if phase === "error"}
                <div
                  role="alert"
                  class={[
                    "relative m-3 flex gap-3 p-4",
                    stopped ? "bg-base-200" : "bg-error/5",
                  ]}
                >
                  <span
                    aria-hidden="true"
                    class={[
                      "hud hud-small pointer-events-none absolute inset-0",
                      !stopped && "hud-lit hud-error",
                    ]}
                  ></span>

                  <Icon
                    icon={stopped ? "lucide:circle-pause" : "lucide:circle-x"}
                    class={[
                      "mt-1 size-4 shrink-0",
                      stopped ? "text-base-content/70" : "text-error",
                    ]}
                  />

                  <div class="flex min-w-0 flex-1 flex-col gap-2">
                    <p
                      class={[
                        "text-sm leading-6 whitespace-pre-wrap wrap-anywhere",
                        "select-text",
                      ]}
                    >
                      {query.error}
                    </p>

                    {#if advice}
                      <p class="text-xs text-base-content/70">{advice}</p>
                    {/if}
                  </div>

                  <button
                    type="button"
                    aria-label={m.menu_copy()}
                    onclick={() =>
                      navigator.clipboard.writeText(query.error ?? "")}
                    class="btn btn-square btn-ghost btn-sm"
                  >
                    <Icon icon="lucide:copy" class="size-4" />
                  </button>
                </div>
              {:else if phase === "chart" && query.result}
                <Lazy
                  load={() => import("@gpql/ui/data/ResultChart.svelte")}
                  props={{
                    columns: query.result.columns,
                    rows: query.result.rows,
                    labels: chartLabels(),
                  }}
                />
              {:else if phase === "grid"}
                {#key query.result}
                  <div in:fade={veil()} class="flex min-h-0 flex-1 flex-col">
                    <ResultGrid result={query.result} empty="" />
                  </div>
                {/key}
              {:else if phase === "touched" && query.result}
                <EmptyState
                  art={null}
                  title={m.rows_touched({ count: query.result.affected ?? 0 })}
                  class="flex-1"
                />
              {:else}
                <EmptyState
                  art={well < rem(16) ? null : "sheet"}
                  title={m.results_empty()}
                  hint={m.results_empty_hint()}
                  class="flex-1"
                />
              {/if}
            </div>
          {/key}
        </div>
      {/if}
    </div>
  </Panel>
</TabLayout>

{#if asking}
  <AskDialog onclose={() => (asking = false)} />
{/if}
