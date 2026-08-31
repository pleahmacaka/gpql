import { invoke } from "@tauri-apps/api/core"
import { Data, Effect } from "effect"

import type {
  BackendInfo,
  CellEdit,
  Completion,
  Credential,
  DbObject,
  Discovery,
  Engine,
  ExportFormat,
  Plan,
  Provider,
  QueryResult,
  QueryShape,
  ReleaseCheck,
  SavedLogin,
  SessionConfig,
  SessionHandle,
  SharedErd,
  Slice,
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
    tunnel: {
      host: "",
      port: "",
      user: "",
      password: "",
      keyPath: "",
      passphrase: "",
      localPort: "",
    },
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

export const resetSessions = () => call<void>("reset_sessions")

export const setReadOnly = (id: string, on: boolean) =>
  call<void>("set_read_only", { id, on })

export const setManual = (id: string, on: boolean) =>
  call<void>("set_manual", { id, on })

export const endTransaction = (id: string, commit: boolean) =>
  call<boolean>("end_transaction", { id, commit })

export const pendingEdits = (id: string, table: string, edits: CellEdit[]) =>
  call<string[]>("pending_edits", { id, table, edits })

export const tables = (id: string) => call<TableInfo[]>("tables", { id })

export const schemas = (id: string) => call<string[]>("schemas", { id })

export const objects = (id: string) => call<DbObject[]>("objects", { id })

export const useSchema = (id: string, name: string) =>
  call<void>("use_schema", { id, name })

export const tableRows = (id: string, table: string, slice: Slice) =>
  call<QueryResult>("table_rows", { id, table, slice })

export const builtQuery = (
  id: string,
  table: string,
  slice: Slice,
  shape: QueryShape,
) => call<string>("built_query", { id, table, slice, shape })

export const exportTable = (
  id: string,
  table: string,
  slice: Slice,
  format: ExportFormat,
  path: string,
) => call<number>("export_table", { id, table, slice, format, path })

export const exportResult = (
  id: string,
  result: QueryResult,
  table: string,
  format: ExportFormat,
  path: string,
) => call<number>("export_result", { id, result, table, format, path })

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

export const tableDdl = (id: string, table: string) =>
  call<string>("table_ddl", { id, table })

export const explainQuery = (id: string, sql: string, analyze: boolean) =>
  call<Plan>("explain_query", { id, sql, analyze })

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

export const setAcrylic = (on: boolean, dark: boolean) =>
  call<void>("set_acrylic", { on, dark })

export const publishSchema = (site: string, name: string, sessionId: string) =>
  call<SharedErd>("publish_schema", { site, name, sessionId })

export const shareErd = (site: string, id: string, open: boolean) =>
  call<boolean>("share_erd", { site, id, open })

export const openLink = (url: string) => call<void>("open_link", { url })

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

export const portFree = (port: number) => call<boolean>("port_free", { port })

export const sshKeys = () => call<string[]>("ssh_keys")

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
