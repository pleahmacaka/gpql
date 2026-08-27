export interface SchemaColumn {
  name: string
  dataType: string
  primaryKey: boolean
  required: boolean
  references: string | null
  note?: string | null
}

export interface SchemaTable {
  name: string
  rows: number
  columns: SchemaColumn[]
  note?: string | null
  hints?: string[]
  policies?: string[]
}

export interface BackendField {
  key: string
  label: string
  placeholder: string
  secret: boolean
}

export interface BackendInfo {
  id: string
  label: string
  dialect: string
  icon: string
  port: string
  fields: BackendField[]
}

export interface SessionHop {
  host: string
  port: string
  user: string
  password: string
  keyPath: string
  passphrase: string
}

export interface SessionDraft {
  kind: string
  host: string
  port: string
  user: string
  password: string
  database: string
  path: string
  url: string
  token: string
  tunnel?: SessionHop
  [key: string]: string | boolean | SessionHop | undefined
}

export interface CredentialPreset {
  name: string
  user: string
  password: string
}

export interface Probe {
  tone: "idle" | "busy" | "good" | "bad"
  text: string
}
