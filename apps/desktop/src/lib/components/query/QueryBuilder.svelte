<script lang="ts">
  import { onMount, type Snippet } from "svelte"
  import { fade } from "svelte/transition"

  import { Dropdown, Icon, Marker, veil } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { FilterOp } from "$lib/types"

  import Code from "./Code.svelte"

  type Props = { built?: string; onuse?: () => void }

  let { built = $bindable(""), onuse }: Props = $props()

  type Clause = { id: number; column: string; op: FilterOp; value: string }

  const OPS: { value: FilterOp; label: string }[] = [
    { value: "eq", label: "=" },
    { value: "ne", label: "<>" },
    { value: "gt", label: ">" },
    { value: "gte", label: ">=" },
    { value: "lt", label: "<" },
    { value: "lte", label: "<=" },
    { value: "contains", label: m.filter_contains() },
    { value: "starts", label: m.builder_starts() },
    { value: "ends", label: m.builder_ends() },
    { value: "isnull", label: m.builder_isnull() },
    { value: "notnull", label: m.builder_notnull() },
  ]

  const VALUELESS: FilterOp[] = ["isnull", "notnull"]

  const RANGES = [
    { value: "0", label: m.range_all() },
    { value: "-15m", label: "15m" },
    { value: "-1h", label: "1h" },
    { value: "-6h", label: "6h" },
    { value: "-24h", label: "24h" },
    { value: "-7d", label: "7d" },
    { value: "-30d", label: "30d" },
  ]

  const WINDOWS = [
    { value: "", label: m.window_raw() },
    { value: "1m", label: "1m" },
    { value: "5m", label: "5m" },
    { value: "15m", label: "15m" },
    { value: "1h", label: "1h" },
    { value: "1d", label: "1d" },
  ]

  const FUNCS = [
    "mean",
    "median",
    "last",
    "first",
    "max",
    "min",
    "sum",
    "count",
  ].map(name => ({ value: name, label: name }))

  let table = $state("")
  let picked = $state<string[]>([])
  let clauses = $state<Clause[]>([])
  let sortColumn = $state("")
  let descending = $state(false)
  let limit = $state(String(workspace.rowLimit))
  let findTable = $state("")
  let findColumn = $state("")
  let range = $state("0")
  let every = $state("")
  let func = $state("mean")
  let reading = $state(true)
  let made = 0

  let timely = $derived(workspace.dialect === "flux")

  let columns = $derived(
    workspace.schema.find(entry => entry.name === table)?.columns ?? [],
  )

  let tables = $derived(
    workspace.tables.filter(entry =>
      entry.name.toLowerCase().includes(findTable.trim().toLowerCase()),
    ),
  )

  let shown = $derived(
    columns.filter(column =>
      column.name.toLowerCase().includes(findColumn.trim().toLowerCase()),
    ),
  )

  let names = $derived(
    columns.map(column => ({ value: column.name, label: column.name })),
  )

  let sorts = $derived([{ value: "", label: m.builder_unsorted() }, ...names])

  let limits = $derived(
    workspace.limits.map(rows => ({
      value: String(rows),
      label: m.rows_count({ count: rows }),
    })),
  )

  let slice = $derived({
    limit: Number(limit),
    offset: 0,
    sort: sortColumn === "" ? null : { column: sortColumn, descending },
    filters: clauses
      .filter(
        clause =>
          clause.column !== "" &&
          (clause.value !== "" || VALUELESS.includes(clause.op)),
      )
      .map(({ column, op, value }) => ({ column, op, value })),
    columns: picked,
  })

  let shape = $derived(
    timely ? { range, every, func } : { range: "", every: "", func: "" },
  )

  onMount(async () => {
    try {
      await workspace.loadSchema()
    } finally {
      reading = false
    }
  })

  $effect(() => {
    if (table === "" && workspace.tables.length > 0) {
      table = workspace.tables[0].name
    }
  })

  $effect(() => {
    const session = workspace.session
    const asked = { table, slice, shape }

    if (!session || asked.table === "") {
      built = ""

      return
    }

    let live = true

    api
      .run(api.builtQuery(session.id, asked.table, asked.slice, asked.shape))
      .then(text => {
        if (live) {
          built = text
        }
      })
      .catch(() => {
        if (live) {
          built = ""
        }
      })

    return () => {
      live = false
    }
  })

  function choose(name: string) {
    table = name
    picked = []
    clauses = []
    sortColumn = ""
    descending = false
    findColumn = ""
  }

  function toggle(name: string) {
    const all = columns.map(column => column.name)
    const current = picked.length === 0 ? all : picked
    const next = current.includes(name)
      ? current.filter(entry => entry !== name)
      : [...current, name]

    picked = next.length === all.length ? [] : next
  }

  function add() {
    clauses = [
      ...clauses,
      { id: made++, column: columns[0]?.name ?? "", op: "eq", value: "" },
    ]
  }

  function drop(id: number) {
    clauses = clauses.filter(clause => clause.id !== id)
  }
</script>

{#snippet search(label: string, term: string, oninput: (v: string) => void)}
  <label class="input input-sm mx-3 mb-2 w-auto shrink-0 bg-base-100">
    <Icon icon="lucide:search" class="size-4 shrink-0 text-base-content/60" />

    <input
      value={term}
      placeholder={label}
      aria-label={label}
      oninput={event => oninput(event.currentTarget.value)}
      class="min-w-0 grow select-text placeholder:text-base-content/60"
    />
  </label>
{/snippet}

{#snippet row(label: string, body: Snippet)}
  <div class="flex min-h-8 flex-wrap items-center gap-2">
    <span class="w-20 shrink-0 text-xs text-base-content/70">{label}</span>
    {@render body()}
  </div>
{/snippet}

<div class="@container flex h-full min-h-0 flex-col">
  <div
    class={[
      "grid min-h-0 flex-1 grid-cols-2 overflow-y-auto",
      "@3xl:grid-cols-3 @3xl:overflow-hidden",
    ]}
  >
    <section
      aria-label={m.builder_from()}
      class={[
        "flex h-56 min-h-0 flex-col border-r border-base-content/10",
        "@3xl:h-auto",
      ]}
    >
      <div class="flex h-10 shrink-0 items-center gap-2 px-3">
        <Marker as="h3" label={m.builder_from()} class="flex-1" />

        <span class="text-xs text-base-content/70 tabular-nums">
          {workspace.tables.length}
        </span>
      </div>

      {@render search(m.builder_find_table(), findTable, v => (findTable = v))}

      <div class="min-h-0 flex-1 overflow-y-auto pb-2">
        {#each tables as entry (entry.name)}
          <button
            type="button"
            onclick={() => choose(entry.name)}
            aria-pressed={entry.name === table}
            class={[
              "relative flex w-full cursor-pointer items-center gap-2 py-1",
              "pr-3 pl-4 text-left text-sm transition-colors",
              entry.name === table
                ? "bg-primary/10 font-medium"
                : "hover:bg-base-content/5",
            ]}
          >
            {#if entry.name === table}
              <span
                aria-hidden="true"
                class="absolute inset-y-0 left-0 w-1 bg-primary"
              ></span>
            {/if}

            <span class="min-w-0 flex-1 truncate">{entry.name}</span>

            <span class="shrink-0 text-xs text-base-content/70 tabular-nums">
              {entry.rows.toLocaleString()}
            </span>
          </button>
        {:else}
          <p class="px-4 py-2 text-xs text-base-content/70">
            {m.builder_no_tables()}
          </p>
        {/each}
      </div>
    </section>

    <section
      aria-label={m.builder_columns()}
      class={[
        "flex h-56 min-h-0 flex-col @3xl:h-auto @3xl:border-r",
        "@3xl:border-base-content/10",
      ]}
    >
      <div class="flex h-10 shrink-0 items-center gap-2 px-3">
        <Marker as="h3" label={m.builder_columns()} class="flex-1" />

        <button
          type="button"
          onclick={() => (picked = [])}
          aria-pressed={picked.length === 0}
          disabled={columns.length === 0}
          class="btn btn-ghost btn-xs font-medium"
        >
          {m.builder_all_columns()}
        </button>
      </div>

      {@render search(
        m.builder_find_column(),
        findColumn,
        v => (findColumn = v),
      )}

      <div class="min-h-0 flex-1 overflow-y-auto pb-2">
        {#if reading && columns.length === 0}
          <p
            in:fade={veil()}
            role="status"
            class={[
              "flex items-center gap-2 px-4 py-2 text-xs",
              "text-base-content/70",
            ]}
          >
            <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
            {m.builder_reading()}
          </p>
        {:else}
          {#each shown as column (column.name)}
            <label
              class={[
                "flex cursor-pointer items-center gap-3 py-1 pr-3 pl-4 text-sm",
                "transition-colors hover:bg-base-content/5",
              ]}
            >
              <input
                type="checkbox"
                checked={picked.length === 0 || picked.includes(column.name)}
                onchange={() => toggle(column.name)}
                class="checkbox checkbox-xs checkbox-primary shrink-0"
              />

              <span class="min-w-0 flex-1 truncate">{column.name}</span>

              <span
                class={[
                  "max-w-28 shrink-0 truncate text-xs",
                  "text-base-content/70",
                ]}
              >
                {column.dataType}
              </span>
            </label>
          {:else}
            <p class="px-4 py-2 text-xs text-base-content/70">
              {m.builder_no_columns()}
            </p>
          {/each}
        {/if}
      </div>
    </section>

    <section
      aria-label={m.builder_filters()}
      class={[
        "col-span-2 flex flex-col gap-4 border-t border-base-content/10 p-3",
        "@3xl:col-span-1 @3xl:overflow-y-auto @3xl:border-t-0",
      ]}
    >
      <div class="flex flex-col gap-2">
        <div class="flex h-6 items-center gap-2">
          <Marker as="h3" label={m.builder_filters()} class="flex-1" />

          <button
            type="button"
            onclick={add}
            disabled={columns.length === 0}
            class="btn btn-ghost btn-xs font-medium"
          >
            <Icon icon="lucide:plus" class="size-4" />
            {m.builder_add_filter()}
          </button>
        </div>

        {#each clauses as clause, at (clause.id)}
          <div
            in:fade={veil()}
            class="flex flex-wrap items-center gap-1 bg-base-200 p-1 hairline"
          >
            <Dropdown
              small
              label={m.builder_filter_column()}
              options={names}
              value={clause.column}
              onpick={next => (clauses[at].column = next)}
            />

            <Dropdown
              small
              label={m.builder_filter_op()}
              options={OPS}
              value={clause.op}
              onpick={next => (clauses[at].op = next)}
            />

            {#if !VALUELESS.includes(clause.op)}
              <input
                bind:value={clauses[at].value}
                placeholder={m.filter_value()}
                aria-label={m.filter_value()}
                class={[
                  "input input-xs min-w-24 flex-1 bg-base-100 select-text",
                  "placeholder:text-base-content/60",
                ]}
              />
            {:else}
              <span class="flex-1"></span>
            {/if}

            <button
              type="button"
              aria-label={m.builder_drop_filter()}
              onclick={() => drop(clause.id)}
              class="btn btn-square btn-ghost btn-xs"
            >
              <Icon icon="lucide:x" class="size-4" />
            </button>
          </div>
        {/each}
      </div>

      <div class="flex flex-col gap-1">
        {#if timely}
          {#snippet rangeBody()}
            <Dropdown
              small
              label={m.range_label()}
              options={RANGES}
              value={range}
              onpick={next => (range = next)}
            />
          {/snippet}

          {@render row(m.range_label(), rangeBody)}

          {#snippet windowBody()}
            <Dropdown
              small
              label={m.window_label()}
              options={WINDOWS}
              value={every}
              onpick={next => (every = next)}
            />

            {#if every !== ""}
              <Dropdown
                small
                label={m.builder_rollup()}
                options={FUNCS}
                value={func}
                onpick={next => (func = next)}
              />
            {/if}
          {/snippet}

          {@render row(m.window_label(), windowBody)}
        {/if}

        {#snippet orderBody()}
          <Dropdown
            small
            label={m.builder_order()}
            options={sorts}
            value={sortColumn}
            onpick={next => (sortColumn = next)}
          />

          {#if sortColumn !== ""}
            <button
              type="button"
              onclick={() => (descending = !descending)}
              aria-pressed={descending}
              class="btn btn-ghost btn-xs font-medium"
            >
              <Icon
                icon={descending ? "lucide:arrow-down" : "lucide:arrow-up"}
                class="size-4"
              />
              {descending ? m.builder_descending() : m.builder_ascending()}
            </button>
          {/if}
        {/snippet}

        {@render row(m.builder_order(), orderBody)}

        {#snippet limitBody()}
          <Dropdown
            small
            label={m.row_limit()}
            options={limits}
            value={limit}
            onpick={next => (limit = next)}
          />
        {/snippet}

        {@render row(m.row_limit(), limitBody)}
      </div>
    </section>
  </div>

  <footer
    aria-label={m.builder_preview()}
    class={[
      "flex h-16 shrink-0 items-start gap-3 border-t border-base-content/10",
      "px-3 py-2",
    ]}
  >
    <div class="min-h-0 min-w-0 flex-1 self-stretch overflow-y-auto">
      <Code code={built} class="text-xs leading-5" />
    </div>

    {#if onuse}
      <button
        type="button"
        onclick={onuse}
        disabled={built === ""}
        class="btn btn-soft btn-sm shrink-0 font-medium"
      >
        <Icon icon="lucide:code" class="size-4" />
        {m.builder_insert()}
      </button>
    {/if}
  </footer>
</div>
