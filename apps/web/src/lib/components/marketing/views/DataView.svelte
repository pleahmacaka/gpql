<script lang="ts">
  import DataGrid from "@gpql/ui/data/DataGrid.svelte"
  import { Icon } from "@gpql/ui/icons/index.ts"

  import { page, schema } from "../sample"

  type Edit = {
    keys: Record<string, string | null>
    set: Record<string, string | null>
  }

  type Props = { active: boolean }

  let { active }: Props = $props()

  const PAGE = 200

  const count = new Intl.NumberFormat("en-US")

  let current = $state("message")
  let rows = $state(page("message", 0, PAGE))
  let spot = $state<{ row: number; column: number } | null>(null)
  let pending = $state<Edit[]>([])

  let table = $derived(
    schema.find(entry => entry.name === current) ?? schema[0],
  )

  let columns = $derived(table.columns.map(entry => entry.name))

  let types = $derived(
    Object.fromEntries(
      table.columns.map(entry => [entry.name, entry.dataType]),
    ),
  )

  let references = $derived(
    Object.fromEntries(
      table.columns.flatMap(entry =>
        entry.references ? [[entry.name, entry.references]] : [],
      ),
    ),
  )

  let statements = $derived(pending.map(edit => statement(edit)))

  function literal(value: string | null) {
    if (value === null) {
      return "null"
    }

    if (value.trim() !== "" && Number.isFinite(Number(value))) {
      return value
    }

    return `'${value.replaceAll("'", "''")}'`
  }

  function statement(edit: Edit) {
    const sets = Object.entries(edit.set)
      .map(([name, value]) => `${name} = ${literal(value)}`)
      .join(", ")

    const keys = Object.entries(edit.keys)
      .map(([name, value]) => `${name} = ${literal(value)}`)
      .join(" and ")

    return `update ${current} set ${sets} where ${keys}`
  }

  function open(name: string, loaded = PAGE) {
    current = name
    rows = page(name, 0, loaded)
    spot = null
    pending = []
  }

  function jump(column: string, value: string) {
    const [target] = references[column].split(".")
    const row = Number(value) - 1

    open(target, Math.ceil((row + 1) / PAGE) * PAGE)
    spot = { row, column: 0 }
  }

  function run() {
    for (const edit of pending) {
      rows = rows.map(row => {
        if (row[0] !== edit.keys.id) {
          return row
        }

        return row.map((cell, index) =>
          columns[index] in edit.set ? edit.set[columns[index]] : cell,
        )
      })
    }

    pending = []
  }
</script>

<div class={["flex h-full gap-2 p-2", active && "live"]}>
  <nav
    aria-label="Tables"
    class="flex w-52 shrink-0 flex-col gap-1 rounded-box bg-base-100 p-2 lift"
  >
    <p class="px-2 py-1 text-xs text-base-content/70">Tables</p>

    {#each schema as entry, index (entry.name)}
      <button
        type="button"
        onclick={() => open(entry.name)}
        aria-pressed={entry.name === current}
        style:animation-delay="{index * 70}ms"
        class={[
          "flex cursor-pointer items-center gap-2 rounded-field px-2 py-2",
          "text-left text-sm transition-colors",
          entry.name === current
            ? "bg-primary/10 text-primary"
            : "hover:bg-base-200",
        ]}
      >
        <Icon icon="lucide:table-2" class="size-4 shrink-0 opacity-60" />
        <span class="min-w-0 flex-1 truncate">{entry.name}</span>
        <span class="text-xs text-base-content/70 tabular-nums">
          {count.format(entry.rows)}
        </span>
      </button>
    {/each}
  </nav>

  <section
    aria-label="Rows of {current}"
    class="relative flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift"
  >
    <header class="flex items-baseline gap-2 px-4 pt-2 pb-1">
      <h3 class="text-sm font-medium">{current}</h3>

      <span class="text-xs text-base-content/70 tabular-nums">
        {count.format(rows.length)} of {count.format(table.rows)}
      </span>
    </header>

    {#key current}
      <DataGrid
        {columns}
        {rows}
        {types}
        {references}
        {spot}
        editable
        wheelPan={false}
        keyColumns={["id"]}
        more={rows.length < table.rows}
        onmore={() => (rows = [...rows, ...page(current, rows.length, PAGE)])}
        onapply={edits => {
          pending = edits
        }}
        onjump={jump}
      />
    {/key}

    {#if pending.length > 0}
      <div
        role="dialog"
        aria-label="Run {statements.length} statement(s)?"
        class="absolute inset-x-4 bottom-14 z-40 rounded-box floating p-4 lift"
      >
        <p class="flex items-center gap-2 text-sm font-medium">
          <Icon icon="lucide:file-pen-line" class="size-4 text-warning" />
          Run {statements.length} statement(s)?
        </p>

        <pre
          class={[
            "mt-2 rounded-field bg-base-200 p-3 text-xs",
            "whitespace-pre-wrap",
          ]}>{statements.join(";\n")};</pre>

        <div class="flex items-center gap-2 pt-3">
          <p class="flex-1 text-xs text-base-content/70">
            Writes to the database straight away.
          </p>

          <button
            type="button"
            class="btn btn-ghost btn-sm"
            onclick={() => (pending = [])}
          >
            Discard
          </button>

          <button type="button" class="btn btn-primary btn-sm" onclick={run}>
            Run
          </button>
        </div>
      </div>
    {/if}
  </section>
</div>

<style>
  .live nav button {
    animation: listed 360ms cubic-bezier(0.2, 0.8, 0.2, 1) backwards;
  }

  @keyframes listed {
    from {
      opacity: 0;
      translate: -0.5rem 0;
    }
  }
</style>
