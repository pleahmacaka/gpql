import type { Engine, TunnelConfig } from "./session"

export interface SavedLogin {
  url: string
  kind: Engine
  host: string
  port: string
  user: string
  password: string
  database: string
  path: string
  endpoint: string
  token: string
  tls: string
  warehouse: string
  schema: string
  tunnel: TunnelConfig
}

export interface Credential {
  name: string
  user: string
  password: string
  builtin: boolean
}

export interface Discovery {
  host: string
  port: string
  user: string
  password: string
  database: string
  needsLogin: boolean
}
