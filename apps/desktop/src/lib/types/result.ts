export interface QueryResult {
  columns: string[]
  rows: (string | null)[][]
  affected: number | null
}

export type FilterOp =
  | "contains"
  | "eq"
  | "ne"
  | "gt"
  | "gte"
  | "lt"
  | "lte"
  | "starts"
  | "ends"
  | "isnull"
  | "notnull"

export interface TableFilter {
  op: FilterOp
  value: string
  needsValue: boolean
}

export interface TableSort {
  column: string
  descending: boolean
}

export interface Slice {
  limit: number
  offset: number
  sort?: TableSort | null
  filters?: { column: string; op: FilterOp; value: string }[]
  columns?: string[]
}

export interface QueryShape {
  range: string
  every: string
  func: string
}

export type ExportFormat = "csv" | "json" | "sql"

export interface PlanNode {
  label: string
  detail: string
  rows: number | null
  cost: number | null
  time: number | null
  children: PlanNode[]
}

export interface Plan {
  tree: PlanNode | null
  text: string
}

export interface CellEdit {
  keys: Record<string, string | null>
  set: Record<string, string | null>
}

export type ObjectKind =
  | "view"
  | "index"
  | "sequence"
  | "routine"
  | "trigger"
  | "type"

export interface DbObject {
  name: string
  kind: ObjectKind
  detail: string
}

export interface TableInfo {
  name: string
  rows: number
}

export interface ColumnInfo {
  name: string
  dataType: string
  primaryKey: boolean
  required: boolean
  references: string | null
  note?: string | null
}

export interface TableSchema {
  name: string
  rows: number
  columns: ColumnInfo[]
  note?: string | null
  hints?: string[]
  policies?: string[]
}

export interface SqlToken {
  start: number
  end: number
  kind: string
}

export interface Completion {
  label: string
  detail: string
  kind: number
}
