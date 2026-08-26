import { invoke } from "@tauri-apps/api/core"
import { Data, Effect } from "effect"

import type {
  BackendInfo,
  Completion,
  Credential,
  Diagnostic,
  Discovery,
  Engine,
  Provider,
  QueryResult,
  ReleaseCheck,
  SavedLogin,
  SessionConfig,
  SessionHandle,
  SqlToken,
  TableInfo,
  TableSchema,
} from "$lib/types"

export class DbError extends Data.TaggedError("DbError")<{
  message: string
}> {}

const call = <A>(command: string, args?: Record<string, unknown>) =>
  Effect.tryPromise({
    try: () => invoke<A>(command, args),
    catch: cause => new DbError({ message: clean(cause) }),
  })

function clean(cause: unknown) {
  const text = String(cause)

  return text.startsWith("Error: ") ? text.slice(7) : text
}

export function blankConfig(kind: Engine = "postgres"): SessionConfig {
  return {
    kind,
    host: "127.0.0.1",
    port: "5432",
    user: "postgres",
    password: "",
    database: "postgres",
    path: "",
    url: "",
    token: "",
    warehouse: "",
    schema: "",
    tls: "prefer",
    readOnly: true,
  }
}

export function describe(config: SessionConfig) {
  if (config.path) {
    return `${config.kind}://${config.path}`
  }

  if (config.url) {
    const bare = config.url.replace(/^https?:\/\//, "").replace(/\/+$/, "")

    return config.database
      ? `${config.kind}://${bare}/${config.database}`
      : `${config.kind}://${bare}`
  }

  const host = config.host || "127.0.0.1"
  const port = config.port || "5432"

  return `${config.kind}://${config.user}@${host}:${port}/${config.database}`
}

export const check = (config: SessionConfig) =>
  call<string>("check", { config }).pipe(
    Effect.timeoutFail({
      duration: "6 seconds",
      onTimeout: () => new DbError({ message: "no answer within 6 seconds" }),
    }),
  )

export const connect = (config: SessionConfig) =>
  call<SessionHandle>("connect", { config })

export const disconnect = (id: string) => call<void>("disconnect", { id })

export const setReadOnly = (id: string, on: boolean) =>
  call<void>("set_read_only", { id, on })

export const tables = (id: string) => call<TableInfo[]>("tables", { id })

export const tableRows = (id: string, table: string, limit = 500, offset = 0) =>
  call<QueryResult>("table_rows", { id, table, limit, offset })

export const applyEdits = (
  id: string,
  table: string,
  edits: {
    keys: Record<string, string | null>
    set: Record<string, string | null>
  }[],
) => call<number>("apply_edits", { id, table, edits })

export const runQuery = (id: string, sql: string) =>
  call<QueryResult>("run_query", { id, sql })

export const schema = (id: string) => call<TableSchema[]>("schema", { id })

export const scanLocal = () =>
  call<Discovery[]>("scan_local").pipe(
    Effect.timeoutTo({
      duration: "20 seconds",
      onTimeout: () => [] as Discovery[],
      onSuccess: found => found,
    }),
  )

export const checkSql = (sql: string, dialect: string) =>
  call<{ line: number; column: number; offset: number; text: string } | null>(
    "check_sql",
    { sql, dialect },
  )

export const highlightSql = (sql: string, dialect: string) =>
  call<SqlToken[]>("highlight_sql", { sql, dialect })

export const lspStart = (dialect: string, program: string, args: string[]) =>
  call<void>("lsp_start", { dialect, program, args })

export const lspStop = (dialect: string) => call<void>("lsp_stop", { dialect })

export const lspRunning = () => call<string[]>("lsp_running")

export const lspComplete = (
  dialect: string,
  text: string,
  line: number,
  character: number,
) => call<Completion[]>("lsp_complete", { dialect, text, line, character })

export const lspDiagnostics = (dialect: string) =>
  call<Diagnostic[]>("lsp_diagnostics", { dialect })

export const setAcrylic = (on: boolean, dark: boolean) =>
  call<void>("set_acrylic", { on, dark })

export const publishSchema = (site: string, name: string, sessionId: string) =>
  call<string>("publish_schema", { site, name, sessionId })

export const savedLogins = () => call<SavedLogin[]>("saved_logins")

export const probeRecents = (items: { url: string; kind: string }[]) =>
  call<string[]>("probe_recents", { items })

export const saveConnection = (config: SessionConfig) =>
  call<string>("save_connection", { config })

export const forgetLogin = (url: string) => call<void>("forget_login", { url })

export const forgetAllLogins = () => call<void>("forget_all_logins")

export const backends = () => call<BackendInfo[]>("backends")

export const credentials = () => call<Credential[]>("credentials")

export const saveCredential = (credential: Omit<Credential, "builtin">) =>
  call<void>("save_credential", {
    credential: { ...credential, builtin: false },
  })

export const forgetCredential = (name: string) =>
  call<void>("forget_credential", { name })

export const providers = () => call<Provider[]>("providers")

export const saveProvider = (provider: Provider) =>
  call<void>("save_provider", { provider })

export const forgetProvider = (id: string) =>
  call<void>("forget_provider", { id })

export const connectOpenrouter = (model: string) =>
  call<Provider>("connect_openrouter", { model })

export const openrouterModels = () => call<string[]>("openrouter_models")

export const latestRelease = () => call<ReleaseCheck>("latest_release")

export const accountToken = () => call<string | null>("account_token")

export const signIn = (site: string) => call<void>("sign_in", { site })

export const forgetAccount = () => call<void>("forget_account")

export const run = <A>(effect: Effect.Effect<A, DbError>) =>
  Effect.runPromise(effect)

export const databases = (config: SessionConfig) =>
  call<string[]>("databases", { config })

export const readDocument = (path: string) =>
  call<string>("read_document", { path })

export const writeDocument = (path: string, text: string) =>
  call<void>("write_document", { path, text })
