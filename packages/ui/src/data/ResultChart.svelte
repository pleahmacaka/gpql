<script lang="ts">
  import {
    AreaChart,
    BarChart,
    LineChart,
    PieChart,
    ScatterChart,
  } from "layerchart"

  import Dropdown from "../controls/Dropdown.svelte"
  import EmptyState from "../controls/EmptyState.svelte"
  import Segmented from "../controls/Segmented.svelte"

  type Labels = {
    bar: string
    line: string
    area: string
    scatter: string
    pie: string
    by: string
    raw: string
    sum: string
    avg: string
    count: string
    empty: string
    shape: string
    axis: string
    value: string
    aggregate: string
    sampled: (shown: number, total: number) => string
  }

  type Props = {
    columns: string[]
    rows: (string | null)[][]
    labels?: Partial<Labels>
  }

  let { columns, rows, labels = {} }: Props = $props()

  let words = $derived<Labels>({
    bar: labels.bar ?? "Bars",
    line: labels.line ?? "Line",
    area: labels.area ?? "Area",
    scatter: labels.scatter ?? "Scatter",
    pie: labels.pie ?? "Pie",
    by: labels.by ?? "by",
    raw: labels.raw ?? "raw",
    sum: labels.sum ?? "sum",
    avg: labels.avg ?? "avg",
    count: labels.count ?? "count",
    empty: labels.empty ?? "No number column to plot in this result",
    shape: labels.shape ?? "Chart type",
    axis: labels.axis ?? "Category column",
    value: labels.value ?? "Value column",
    aggregate: labels.aggregate ?? "Aggregate",
    sampled:
      labels.sampled ?? ((shown, total) => `${shown} of ${total} shown`),
  })

  const LIMIT = 400
  const TICKS = 8
  const LABEL = 12
  const SAMPLE = 200
  const SLICES = 20

  function spread<T>(list: T[], cap: number) {
    if (list.length <= cap) {
      return list
    }

    const step = list.length / cap

    return Array.from({ length: cap }, (_, at) => list[Math.floor(at * step)])
  }

  const blank = (cell: string | null | undefined) =>
    cell == null || cell.trim() === ""

  function numberIn(column: number, sample: (string | null)[][]) {
    let seen = false

    for (const row of sample) {
      const cell = row[column]

      if (blank(cell)) {
        continue
      }

      if (!Number.isFinite(Number(cell))) {
        return false
      }

      seen = true
    }

    return seen
  }

  let indices = $derived(columns.map((_, at) => at))

  let numeric = $derived.by(() => {
    const sample = spread(rows, SAMPLE)

    return indices.filter(at => numberIn(at, sample))
  })

  type Shape = "bar" | "line" | "area" | "scatter" | "pie"
  type Aggregate = "none" | "sum" | "avg" | "count"

  const SHAPES: Shape[] = ["bar", "line", "area", "scatter", "pie"]
  const AGGREGATES: Aggregate[] = ["none", "sum", "avg", "count"]

  let x = $state(-1)
  let y = $state(-1)
  let shape = $state<Shape>("bar")
  let aggregate = $state<Aggregate>("none")

  let shapes = $derived([
    { value: "bar", label: words.bar, icon: "lucide:chart-column" },
    { value: "line", label: words.line, icon: "lucide:chart-line" },
    { value: "area", label: words.area, icon: "lucide:chart-area" },
    { value: "scatter", label: words.scatter, icon: "lucide:chart-scatter" },
    { value: "pie", label: words.pie, icon: "lucide:chart-pie" },
  ])

  let aggregates = $derived([
    { value: "none", label: words.raw },
    { value: "sum", label: words.sum },
    { value: "avg", label: words.avg },
    { value: "count", label: words.count },
  ])

  let raw = $derived(aggregate === "none" || shape === "scatter")
  let axisPool = $derived(shape === "scatter" ? numeric : indices)

  let axis = $derived(
    axisPool.includes(x)
      ? x
      : (axisPool.find(at => !numeric.includes(at)) ?? axisPool[0] ?? -1),
  )

  let value = $derived(numeric.includes(y) ? y : (numeric[0] ?? -1))

  let points = $derived.by(() => {
    if (axis === -1 || value === -1) {
      return []
    }

    const scatter = shape === "scatter"
    const out: { label: string | number; amount: number }[] = []

    for (const row of raw ? spread(rows, LIMIT) : rows) {
      const amount = Number(row[value])
      const label = row[axis]

      if (blank(row[value]) || !Number.isFinite(amount)) {
        continue
      }

      if (scatter && (blank(label) || !Number.isFinite(Number(label)))) {
        continue
      }

      out.push({ label: scatter ? Number(label) : (label ?? "null"), amount })
    }

    return out
  })

  let buckets = $derived.by(() => {
    if (raw) {
      return points
    }

    const held = new Map<string, { total: number; count: number }>()

    for (const point of points) {
      const key = String(point.label)
      const entry = held.get(key) ?? { total: 0, count: 0 }

      entry.total += point.amount
      entry.count += 1
      held.set(key, entry)
    }

    return [...held].map(([label, entry]) => ({
      label,
      amount:
        aggregate === "sum"
          ? entry.total
          : aggregate === "avg"
            ? entry.total / entry.count
            : entry.count,
    }))
  })

  let data = $derived(raw ? buckets : buckets.slice(0, LIMIT))

  function sparse(scale: { domain: () => unknown[] }) {
    const all = scale.domain()
    const step = Math.max(Math.ceil(all.length / TICKS), 1)

    return all.filter((_, at) => at % step === 0)
  }

  const clip = (value: unknown) => {
    const text = String(value)

    return text.length > LABEL ? `${text.slice(0, LABEL - 1)}…` : text
  }

  const axes = { xAxis: { ticks: sparse, format: clip } }

  let hidden = $derived(
    raw
      ? { shown: Math.min(rows.length, LIMIT), total: rows.length }
      : { shown: data.length, total: buckets.length },
  )

  let slices = $derived.by(() => {
    const held = new Map<string, number>()

    for (const point of data) {
      const key = String(point.label)

      held.set(key, (held.get(key) ?? 0) + point.amount)
    }

    return [...held]
      .map(([label, amount]) => ({ label, amount }))
      .sort((a, b) => b.amount - a.amount)
      .slice(0, SLICES)
  })
</script>

<div class="flex min-h-0 flex-1 flex-col gap-3 px-4 pb-4">
  <div class="flex flex-wrap items-center gap-2 text-xs">
    <Segmented
      small
      label={words.shape}
      options={shapes}
      value={shape}
      onpick={next => (shape = SHAPES.find(entry => entry === next) ?? shape)}
    />

    <Dropdown
      small
      label={words.axis}
      value={String(axis)}
      options={axisPool.map(at => ({ value: String(at), label: columns[at] }))}
      onpick={next => (x = Number(next))}
    />

    <span class="text-base-content/70">{words.by}</span>

    <Dropdown
      small
      label={words.value}
      value={String(value)}
      options={numeric.map(at => ({ value: String(at), label: columns[at] }))}
      onpick={next => (y = Number(next))}
    />

    {#if shape !== "scatter"}
      <Dropdown
        small
        label={words.aggregate}
        value={aggregate}
        options={aggregates}
        onpick={next =>
          (aggregate = AGGREGATES.find(entry => entry === next) ?? aggregate)}
      />
    {/if}

    {#if hidden.shown < hidden.total}
      <span class="ml-auto text-base-content/70 tabular-nums">
        {words.sampled(hidden.shown, hidden.total)}
      </span>
    {/if}
  </div>

  <div class="relative min-h-0 flex-1 bg-base-100 p-4 hairline">
    {#if data.length === 0}
      <div class="absolute inset-0 grid place-items-center">
        <EmptyState art="graph" title={words.empty} />
      </div>
    {:else if shape === "bar"}
      <BarChart {data} x="label" y="amount" props={axes} />
    {:else if shape === "line"}
      <LineChart {data} x="label" y="amount" props={axes} />
    {:else if shape === "area"}
      <AreaChart {data} x="label" y="amount" props={axes} />
    {:else if shape === "scatter"}
      <ScatterChart {data} x="label" y="amount" />
    {:else}
      <PieChart data={slices} key="label" value="amount" />
    {/if}
  </div>
</div>
