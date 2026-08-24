export interface QueryResult {
  columns: string[]
  rows: (string | null)[][]
  affected: number | null
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

export interface Diagnostic {
  line: number
  character: number
  endLine: number
  endCharacter: number
  message: string
  severity: number
}
