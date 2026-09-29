export type Chapter = {
  id: "connect" | "data" | "query" | "schema"
  title: string
  line: string
  hint: string
  chips: string[]
  tab: string | null
}

export const CHAPTERS: Chapter[] = [
  {
    id: "connect",
    title: "Connect",
    line: "Finds the databases already running.",
    hint: "Pick a server, then Connect",
    chips: ["Local ports", "Docker", "Tailnet", "SSH tunnel"],
    tab: null,
  },
  {
    id: "data",
    title: "Data",
    line: "Edit a cell, read the SQL, then run it.",
    hint: "Double-click a cell",
    chips: ["Server-side sort", "Pages on scroll", "Manual commit"],
    tab: "Data",
  },
  {
    id: "query",
    title: "Query",
    line: "Describe it, run it, chart it.",
    hint: "Press Run, then Graph",
    chips: ["Tree-sitter", "EXPLAIN tree", "Your own AI key"],
    tab: "Query",
  },
  {
    id: "schema",
    title: "Schema",
    line: "Foreign keys, laid out for you.",
    hint: "Drag a table around",
    chips: ["Auto arrange", "Diff to migration", "Share as a room"],
    tab: "Schema",
  },
]
