<script lang="ts">
  type Props = { code: string }

  let { code }: Props = $props()

  const KEYWORDS = new Set([
    "select",
    "from",
    "join",
    "left",
    "inner",
    "on",
    "where",
    "group",
    "order",
    "by",
    "desc",
    "asc",
    "limit",
    "and",
    "or",
    "not",
    "as",
    "interval",
    "now",
  ])

  const pattern =
    /('[^']*'|--[^\n]*|\b\d+(?:\.\d+)?\b|[A-Za-z_][A-Za-z0-9_]*|[^A-Za-z0-9_\s]+|\s+)/g

  function tone(piece: string, next: string): string {
    if (piece.startsWith("'")) {
      return "tok-string"
    }

    if (piece.startsWith("--")) {
      return "tok-comment"
    }

    if (/^\d/.test(piece)) {
      return "tok-number"
    }

    if (/^[A-Za-z_]/.test(piece)) {
      if (KEYWORDS.has(piece.toLowerCase())) {
        return "tok-keyword"
      }

      return next.startsWith("(") ? "tok-function" : "tok-field"
    }

    if (/^\s+$/.test(piece)) {
      return ""
    }

    return "tok-punctuation"
  }

  let pieces = $derived.by(() => {
    const raw = code.match(pattern) ?? []

    return raw.map((piece, index) => ({
      text: piece,
      kind: tone(piece, raw[index + 1] ?? ""),
    }))
  })
</script>

<pre
  class="flex-1 overflow-auto rounded-field bg-base-200 p-4 font-mono text-xs
    leading-relaxed text-base-content/80">{#each pieces as piece, index (index)}<span
      class={piece.kind}>{piece.text}</span>{/each}</pre>

<style>
  .tok-keyword {
    color: var(--color-info);
  }

  .tok-string {
    color: var(--color-secondary-content);
  }

  .tok-number {
    color: var(--color-warning);
  }

  .tok-function {
    color: var(--color-primary);
  }

  .tok-field {
    color: var(--color-accent);
  }

  .tok-comment {
    color: color-mix(in oklch, currentColor 45%, transparent);
    font-style: italic;
  }

  .tok-punctuation {
    color: color-mix(in oklch, currentColor 60%, transparent);
  }
</style>
