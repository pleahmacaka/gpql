import * as m from "$lib/paraglide/messages"

export function chartLabels() {
  return {
    bar: m.chart_bar(),
    line: m.chart_line(),
    area: m.chart_area(),
    scatter: m.chart_scatter(),
    pie: m.chart_pie(),
    by: m.chart_by(),
    raw: m.chart_raw(),
    sum: m.chart_sum(),
    avg: m.chart_avg(),
    count: m.chart_count(),
    empty: m.chart_empty(),
    shape: m.chart_shape(),
    axis: m.chart_axis(),
    value: m.chart_value(),
    aggregate: m.chart_aggregate(),
    sampled: (shown: number, total: number) =>
      m.chart_sampled({ shown, total }),
  }
}
