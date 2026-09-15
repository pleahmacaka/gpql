import * as m from "$lib/paraglide/messages"

export type Nouns = {
  count: (count: number) => string
  search: () => string
  pick: () => string
  none: () => string
  columns: (count: number) => string
  panel: () => string
  row: (count: number) => string
  rowsLoaded: (loaded: number, total: number) => string
  rowsAll: (loaded: number) => string
  rowsFiltered: (loaded: number) => string
  emptyRows: () => string
}

const plain: Nouns = {
  count: count => m.tables_count({ count }),
  search: () => m.search_tables(),
  pick: () => m.pick_table(),
  none: () => m.no_table(),
  columns: count => m.columns_count({ count }),
  panel: () => m.panel_tables(),
  row: count => m.rows_count({ count }),
  rowsLoaded: (loaded, total) => m.rows_loaded({ loaded, total }),
  rowsAll: loaded => m.rows_all({ loaded }),
  rowsFiltered: loaded => m.rows_filtered({ loaded }),
  emptyRows: () => m.no_rows(),
}

const mqtt: Nouns = {
  count: count => m.topics_count({ count }),
  search: () => m.search_topics(),
  pick: () => m.pick_topic(),
  none: () => m.no_topic(),
  columns: count => m.fields_count({ count }),
  panel: () => m.panel_topics(),
  row: count => m.messages_count({ count }),
  rowsLoaded: (loaded, total) => m.messages_loaded({ loaded, total }),
  rowsAll: loaded => m.messages_all({ loaded }),
  rowsFiltered: loaded => m.messages_filtered({ loaded }),
  emptyRows: () => m.no_messages(),
}

export function nounsFor(kind: string): Nouns {
  return kind === "mqtt" ? mqtt : plain
}
