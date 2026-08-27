<script lang="ts">
  import {
    AreaChart,
    BarChart,
    LineChart,
    PieChart,
    ScatterChart,
  } from "layerchart"

  type Props = {
    columns: string[]
    rows: (string | null)[][]
  }

  let { columns, rows }: Props = $props()

  const LIMIT = 400
  const SLICES = 20

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
  let shape = $state<"bar" | "line" | "area" | "scatter" | "pie">("bar")
  let aggregate = $state<"none" | "sum" | "avg" | "count">("none")

  let axisPool = $derived(shape === "scatter" ? numeric : columns)

  let axis = $derived.by(() => {
    if (axisPool.includes(x)) {
      return x
    }

    const fallback = axisPool.find(name => labels.includes(name))

    return fallback ?? axisPool[0] ?? ""
  })
  let value = $derived(numeric.includes(y) ? y : (numeric[0] ?? ""))

  $effect(() => {
    x = axis
  })

  $effect(() => {
    y = value
  })

  let points = $derived.by(() => {
    const left = columns.indexOf(axis)
    const right = columns.indexOf(value)

    if (left === -1 || right === -1) {
      return []
    }

    return rows.slice(0, LIMIT).map(row => ({
      label:
        shape === "scatter" ? Number(row[left] ?? 0) : (row[left] ?? "null"),
      amount: Number(row[right] ?? 0),
    }))
  })

  let data = $derived.by(() => {
    if (aggregate === "none" || shape === "scatter") {
      return points
    }

    const buckets = new Map<string, { total: number; count: number }>()

    for (const point of points) {
      const key = String(point.label)
      const held = buckets.get(key) ?? { total: 0, count: 0 }

      held.total += point.amount
      held.count += 1
      buckets.set(key, held)
    }

    return [...buckets].map(([label, held]) => ({
      label,
      amount:
        aggregate === "sum"
          ? held.total
          : aggregate === "avg"
            ? held.total / held.count
            : held.count,
    }))
  })

  let slices = $derived(
    [...data].sort((a, b) => b.amount - a.amount).slice(0, SLICES),
  )
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <div class="flex flex-wrap items-center gap-2 px-4 pb-2">
    <select
      bind:value={shape}
      class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs outline-none"
    >
      <option value="bar">Bars</option>
      <option value="line">Line</option>
      <option value="area">Area</option>
      <option value="scatter">Scatter</option>
      <option value="pie">Pie</option>
    </select>

    <select
      value={axis}
      onchange={event => (x = event.currentTarget.value)}
      class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs outline-none"
    >
      {#each axisPool as name (name)}
        <option value={name}>{name}</option>
      {/each}
    </select>

    <span class="text-xs text-base-content/40">by</span>

    <select
      value={value}
      onchange={event => (y = event.currentTarget.value)}
      class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs outline-none"
    >
      {#each numeric as name (name)}
        <option value={name}>{name}</option>
      {/each}
    </select>

    {#if shape !== "scatter"}
      <select
        bind:value={aggregate}
        class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs outline-none"
      >
        <option value="none">raw</option>
        <option value="sum">sum</option>
        <option value="avg">avg</option>
        <option value="count">count</option>
      </select>
    {/if}

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
    {:else if shape === "line"}
      <LineChart {data} x="label" y="amount" />
    {:else if shape === "area"}
      <AreaChart {data} x="label" y="amount" />
    {:else if shape === "scatter"}
      <ScatterChart {data} x="label" y="amount" />
    {:else}
      <PieChart data={slices} key="label" value="amount" />
    {/if}
  </div>
</div>
