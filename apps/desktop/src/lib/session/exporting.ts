import { save } from "@tauri-apps/plugin-dialog"

import * as m from "$lib/paraglide/messages"
import type { ExportFormat, QueryResult, Slice } from "$lib/types"

import * as api from "./commands"

const SUFFIX: Record<ExportFormat, string> = {
  csv: "csv",
  json: "json",
  sql: "sql",
}

export const FORMATS: ExportFormat[] = ["csv", "json", "sql"]

async function askWhere(stem: string, format: ExportFormat) {
  return save({
    defaultPath: `${stem}.${SUFFIX[format]}`,
    filters: [{ name: format.toUpperCase(), extensions: [SUFFIX[format]] }],
  })
}

// the whole table streams through the backend, so a filtered million-row
// export never has to exist in the webview
export async function exportTable(
  sessionId: string,
  table: string,
  slice: Slice,
  format: ExportFormat,
) {
  const path = await askWhere(table, format)

  if (!path) {
    return null
  }

  const rows = await api.run(
    api.exportTable(sessionId, table, { ...slice, limit: 0 }, format, path),
  )

  return m.export_done({ rows, path })
}

export async function exportResult(
  sessionId: string,
  result: QueryResult,
  format: ExportFormat,
) {
  const path = await askWhere("result", format)

  if (!path) {
    return null
  }

  const rows = await api.run(
    api.exportResult(sessionId, result, "result", format, path),
  )

  return m.export_done({ rows, path })
}
