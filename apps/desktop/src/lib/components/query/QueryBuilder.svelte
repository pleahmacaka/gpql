<script lang="ts">
  import { Dropdown, Icon } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { FilterOp } from "$lib/types"

  type Clause = { column: string; op: FilterOp; value: string }

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

  let table = $state("")
  let picked = $state<string[]>([])
  let clauses = $state<Clause[]>([])
  let sortColumn = $state("")
  let descending = $state(false)
  let built = $state("")

  let findTable = $state("")
  let findColumn = $state("")

  let range = $state("0")
  let every = $state("")
  let func = $state("mean")

  let timely = $derived(workspace.dialect === "flux")

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

  const FUNCS = ["mean", "median", "last", "first", "max", "min", "sum", "count"]
    .map(name => ({ value: name, label: name }))

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

  let sorts = $derived([
    { value: "", label: m.builder_unsorted() },
    ...columns.map(column => ({ value: column.name, label: column.name })),
  ])

  $effect(() => {
    if (table === "" && workspace.tables.length > 0) {
      table = workspace.tables[0].name
    }
  })

  let slice = $derived({
    limit: workspace.rowLimit,
    offset: 0,
    sort: sortColumn === "" ? null : { column: sortColumn, descending },
    filters: clauses.filter(
      clause =>
        clause.column !== "" &&
        (clause.value !== "" || VALUELESS.includes(clause.op)),
    ),
    columns: picked,
  })

  let shape = $derived(
    timely ? { range, every, func } : { range: "", every: "", func: "" },
  )

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
          workspace.query.sql = text
          workspace.query.selection = { start: 0, end: 0 }
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
    picked = picked.includes(name)
      ? picked.filter(entry => entry !== name)
      : [...picked, name]
  }

  function add() {
    clauses = [
      ...clauses,
      { column: columns[0]?.name ?? "", op: "eq", value: "" },
    ]
  }

  function drop(at: number) {
    clauses = clauses.filter((_, index) => index !== at)
  }

</script>

{#snippet search(placeholder: string, term: string, oninput: (v: string) => void)}
  <input
    value={term}
    {placeholder}
    oninput={event => oninput(event.currentTarget.value)}
    class="mx-2 mb-1 rounded-field bg-base-100 px-2 py-1 text-xs outline-none
      select-text placeholder:text-base-content/30"
  />
{/snippet}

<div class="mx-3 mb-2 space-y-2 text-xs">
  <div class="grid grid-cols-2 gap-2">
    <section class="flex h-44 flex-col rounded-field bg-base-200 pt-2">
      <h3 class="px-3 pb-2 text-base-content/40">{m.builder_from()}</h3>

      {@render search(m.builder_find_table(), findTable, v => (findTable = v))}

      <div class="flex-1 overflow-y-auto px-2 pb-2">
        {#each tables as entry (entry.name)}
          <button
            type="button"
            onclick={() => choose(entry.name)}
            aria-pressed={entry.name === table}
            class="flex w-full items-center gap-2 rounded-field px-2 py-1
              text-left {entry.name === table
              ? 'bg-primary/10 text-primary'
              : 'hover:bg-base-300'}"
          >
            <span class="truncate">{entry.name}</span>
          </button>
        {/each}
      </div>
    </section>

    <section class="flex h-44 flex-col rounded-field bg-base-200 pt-2">
      <h3 class="flex items-center gap-2 px-3 pb-2 text-base-content/40">
        {m.builder_columns()}

        <span class="flex-1"></span>

        <button
          type="button"
          onclick={() => (picked = [])}
          class="rounded-field px-2 {picked.length === 0
            ? 'text-primary'
            : 'hover:text-base-content'}"
        >
          {m.builder_all_columns()}
        </button>
      </h3>

      {@render search(m.builder_find_column(), findColumn, v => (findColumn = v))}

      <div class="flex-1 overflow-y-auto px-2 pb-2">
        {#each shown as column (column.name)}
          <button
            type="button"
            onclick={() => toggle(column.name)}
            aria-pressed={picked.includes(column.name)}
            class="flex w-full items-center gap-2 rounded-field px-2 py-1
              text-left {picked.includes(column.name)
              ? 'bg-primary/10 text-primary'
              : 'hover:bg-base-300'}"
          >
            <Icon
              icon={picked.includes(column.name)
                ? "lucide:check"
                : "lucide:minus"}
              class="size-4 shrink-0 {picked.includes(column.name)
                ? ''
                : 'text-base-content/20'}"
            />

            <span class="truncate">{column.name}</span>

            <span class="flex-1"></span>

            <span class="shrink-0 truncate text-base-content/30">
              {column.dataType}
            </span>
          </button>
        {:else}
          <p class="px-2 py-1 text-base-content/35">{m.builder_no_columns()}</p>
        {/each}
      </div>
    </section>
  </div>

  <div class="space-y-2 rounded-field bg-base-200 px-3 py-2">
    <div class="flex items-center gap-2">
      <span class="text-base-content/40">{m.builder_filters()}</span>

      <span class="flex-1"></span>

      <button
        type="button"
        onclick={add}
        disabled={columns.length === 0}
        class="flex items-center gap-2 rounded-field bg-base-100 px-2 py-1
          hover:bg-base-300 disabled:opacity-40"
      >
        <Icon icon="lucide:plus" class="size-4" />
        {m.builder_add_filter()}
      </button>
    </div>

    {#each clauses as clause, at (at)}
      <div class="flex items-center gap-2">
        <Dropdown
          options={columns.map(column => ({
            value: column.name,
            label: column.name,
          }))}
          value={clause.column}
          onpick={next => (clauses[at].column = next)}
        />

        <Dropdown
          options={OPS}
          value={clause.op}
          onpick={next => (clauses[at].op = next)}
        />

        {#if !VALUELESS.includes(clause.op)}
          <input
            bind:value={clauses[at].value}
            placeholder={m.filter_value()}
            class="min-w-0 flex-1 rounded-field bg-base-100 px-2 py-1 outline-none
              select-text placeholder:text-base-content/30"
          />
        {:else}
          <span class="flex-1"></span>
        {/if}

        <button
          type="button"
          aria-label={m.builder_drop_filter()}
          onclick={() => drop(at)}
          class="grid size-6 shrink-0 place-items-center rounded-selector
            text-base-content/40 hover:text-error"
        >
          <Icon icon="lucide:x" class="size-4" />
        </button>
      </div>
    {/each}
  </div>

  <div
    class="flex flex-wrap items-center gap-2 rounded-field bg-base-200 px-3 py-2"
  >
    {#if timely}
      <Icon icon="lucide:clock" class="size-4 text-base-content/40" />

      <span class="text-base-content/40">{m.range_label()}</span>

      <Dropdown options={RANGES} value={range} onpick={next => (range = next)} />

      <span class="pl-2 text-base-content/40">{m.window_label()}</span>

      <Dropdown options={WINDOWS} value={every} onpick={next => (every = next)} />

      {#if every !== ""}
        <Dropdown options={FUNCS} value={func} onpick={next => (func = next)} />
      {/if}

      <span class="w-full"></span>
    {/if}

    <Icon icon="lucide:arrow-up-down" class="size-4 text-base-content/40" />

    <span class="text-base-content/40">{m.builder_order()}</span>

    <Dropdown
      options={sorts}
      value={sortColumn}
      onpick={next => (sortColumn = next)}
    />

    {#if sortColumn !== ""}
      <button
        type="button"
        onclick={() => (descending = !descending)}
        class="flex items-center gap-2 rounded-field bg-base-100 px-2 py-1
          hover:bg-base-300"
      >
        <Icon
          icon={descending ? "lucide:arrow-down" : "lucide:arrow-up"}
          class="size-4"
        />
        {descending ? m.builder_descending() : m.builder_ascending()}
      </button>
    {/if}
  </div>

  <pre
    class="overflow-x-auto rounded-field bg-base-200 px-3 py-2 font-mono
      text-base-content/70">{built}</pre>
</div>
