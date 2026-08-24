export type Engine = string

export type Tab = "data" | "query" | "schema"

export type Mode = "new" | "quick" | "recent" | "erd"

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

export interface SessionConfig {
  kind: Engine
  host: string
  port: string
  user: string
  password: string
  database: string
  path: string
  url: string
  token: string
  tls: string
  readOnly: boolean
  [key: string]: string | boolean
}

export interface SessionHandle {
  id: string
  label: string
  detail: string
  kind: Engine
  readOnly: boolean
}

export interface Probe {
  tone: "idle" | "busy" | "good" | "bad"
  text: string
}
