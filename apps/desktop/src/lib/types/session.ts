export type Engine = string

export type Tab = "data" | "query" | "schema"

export type Mode = "new" | "quick" | "recent" | "erd"

export type SchemaState = "idle" | "loading" | "ready" | "failed"

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
  wip: boolean
  explain: boolean
  analyze: boolean
}

export interface SharedErd {
  id: string
  link: string
  open: boolean
}

export interface ErdRoom {
  id: string
  name: string
  open: boolean
  createdAt: string | number | null
  link: string
}

export interface TunnelConfig {
  host: string
  port: string
  user: string
  password: string
  keyPath: string
  passphrase: string
  localPort: string
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
  warehouse: string
  schema: string
  tls: string
  readOnly: boolean
  create?: boolean
  tunnel?: TunnelConfig
  [key: string]: string | boolean | TunnelConfig | undefined
}

export interface SessionHandle {
  id: string
  label: string
  detail: string
  kind: Engine
  readOnly: boolean
  sliceable: boolean
  transactional: boolean
}

export interface Subscription {
  filter: string
  qos: number
}

export interface Probe {
  tone: "idle" | "busy" | "good" | "bad"
  text: string
}

export interface ReleaseCheck {
  current: string
  latest: string
  link: string
  fresh: boolean
}
