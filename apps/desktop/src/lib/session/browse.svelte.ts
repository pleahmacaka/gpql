import type {
  QueryResult,
  SessionHandle,
  Slice,
  TableFilter,
  TableSort,
} from "$lib/types"

import * as api from "./commands"

export type BrowseHost = {
  session: () => SessionHandle | null
  pageSize: () => number
  onCount: (table: string, rows: number) => void
}

export class Browse {
  table = $state<string | null>(null)
  result = $state<QueryResult | null>(null)

  sort = $state<TableSort | null>(null)
  filters = $state<Record<string, TableFilter>>({})

  busy = $state(false)
  paging = $state(false)
  end = $state(false)
  error = $state<string | null>(null)

  private host: BrowseHost
  private run = 0

  constructor(host: BrowseHost) {
    this.host = host
  }

  // a page-at-a-time grid can only rank what it has, so the server has to be
  // the one that orders and filters or the view quietly lies about the table
  get serverSide() {
    return this.host.session()?.sliceable ?? false
  }

  slice(offset: number): Slice {
    const limit = this.host.pageSize()

    if (!this.serverSide) {
      return { limit, offset }
    }

    return {
      limit,
      offset,
      sort: this.sort,
      filters: Object.entries(this.filters)
        .filter(([, filter]) => filter.value !== "" || !filter.needsValue)
        .map(([column, filter]) => ({
          column,
          op: filter.op,
          value: filter.value,
        })),
    }
  }

  exportSlice(): Slice {
    return { ...this.slice(0), limit: 0 }
  }

  reset() {
    this.table = null
    this.result = null
    this.sort = null
    this.filters = {}
    this.end = false
    this.error = null
  }

  async open(table: string) {
    if (table !== this.table) {
      this.sort = null
      this.filters = {}
    }

    this.table = table
    await this.reload()
  }

  async reload() {
    const session = this.host.session()
    const table = this.table

    if (!session || !table) {
      return
    }

    const ticket = ++this.run

    this.busy = true
    this.error = null

    try {
      const page = await api.run(
        api.tableRows(session.id, table, this.slice(0)),
      )

      if (ticket !== this.run) {
        return
      }

      this.result = page
      this.end = page.rows.length < this.host.pageSize()

      if (this.end && this.untouched) {
        this.host.onCount(table, page.rows.length)
      }
    } catch (failure) {
      if (ticket === this.run) {
        this.error = String(failure)
        this.result = null
      }
    } finally {
      if (ticket === this.run) {
        this.busy = false
      }
    }
  }

  // a filtered page count would be wrong as the table's own total
  private get untouched() {
    return this.sort === null && Object.keys(this.filters).length === 0
  }

  async more() {
    const session = this.host.session()
    const table = this.table
    const held = this.result

    if (!session || !table || !held || this.end || this.paging || this.busy) {
      return
    }

    const ticket = this.run

    this.paging = true

    try {
      const page = await api.run(
        api.tableRows(session.id, table, this.slice(held.rows.length)),
      )

      if (ticket !== this.run || this.result !== held) {
        return
      }

      this.result = { ...held, rows: [...held.rows, ...page.rows] }
      this.end = page.rows.length < this.host.pageSize()
    } catch (failure) {
      this.error = String(failure)
      this.end = true
    } finally {
      this.paging = false
    }
  }

  async setSort(sort: TableSort | null) {
    this.sort = sort

    if (this.serverSide) {
      await this.reload()
    }
  }

  async setFilters(filters: Record<string, TableFilter>) {
    this.filters = filters

    if (this.serverSide) {
      await this.reload()
    }
  }
}
