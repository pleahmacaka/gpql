<script lang="ts">
  import { BarChart, LineChart } from "layerchart"

  type Props = {
    columns: string[]
    rows: (string | null)[][]
  }

  let { columns, rows }: Props = $props()

  const LIMIT = 400

  let numeric = $derived(
    columns.filter((_, index) =>
      rows
        .slice(0, 20)
        .some(row => row[index] !== null && !Number.isNaN(Number(row[index]))),
    ),
  )

  let labels = $derived(columns.filter(name => !numeric.includes(name)))

  let x = $state("")
  let y = $state("")
  let shape = $state<"bar" | "line">("bar")

  let axis = $derived(x || labels[0] || columns[0] || "")
  let value = $derived(y || numeric[0] || "")

  let data = $derived.by(() => {
    const left = columns.indexOf(axis)
    const right = columns.indexOf(value)

    if (left === -1 || right === -1) {
      return []
    }

    return rows.slice(0, LIMIT).map(row => ({
      label: row[left] ?? "null",
      amount: Number(row[right] ?? 0),
    }))
  })
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <div class="flex flex-wrap items-center gap-2 px-4 pb-2">
    <select
      bind:value={shape}
      class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs outline-none"
    >
      <option value="bar">Bars</option>
      <option value="line">Line</option>
    </select>

    <select
      bind:value={x}
      class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs outline-none"
    >
      {#each columns as name (name)}
        <option value={name}>{name}</option>
      {/each}
    </select>

    <span class="text-xs text-base-content/40">by</span>

    <select
      bind:value={y}
      class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs outline-none"
    >
      {#each numeric as name (name)}
        <option value={name}>{name}</option>
      {/each}
    </select>

    {#if rows.length > LIMIT}
      <span class="text-xs text-base-content/40">
        first {LIMIT} of {rows.length} rows
      </span>
    {/if}
  </div>

  <div class="min-h-0 flex-1 px-4 pb-4">
    {#if data.length === 0 || numeric.length === 0}
      <p class="py-6 text-sm text-base-content/40">
        no number column to plot in this result
      </p>
    {:else if shape === "bar"}
      <BarChart {data} x="label" y="amount" />
    {:else}
      <LineChart {data} x="label" y="amount" />
    {/if}
  </div>
</div>
