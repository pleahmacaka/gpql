import { createOpenAICompatible } from "@ai-sdk/openai-compatible"

import type { Provider, TableSchema } from "$lib/types"

export function modelOf(provider: Provider) {
  const gateway = createOpenAICompatible({
    name: provider.name,
    baseURL: provider.baseUrl,
    apiKey: provider.key,
  })

  return gateway(provider.model)
}

export function outline(tables: TableSchema[]) {
  return tables
    .map(table => {
      const columns = table.columns
        .map(column =>
          column.references
            ? `${column.name} ${column.dataType} -> ${column.references}`
            : `${column.name} ${column.dataType}`,
        )
        .join(", ")

      return `${table.name}(${columns})`
    })
    .join("\n")
}
