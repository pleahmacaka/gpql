<script lang="ts">
  type Props = { code: string; caret?: boolean }

  let { code, caret = false }: Props = $props()

  const KEYWORDS = new Set([
    "select",
    "from",
    "join",
    "on",
    "where",
    "group",
    "order",
    "by",
    "desc",
    "limit",
    "as",
    "interval",
  ])

  const TOKEN = /'[^']*'?|\d+|\w+|\s+|[^\w\s']+/g

  function tone(piece: string, next: string) {
    if (piece.startsWith("'")) {
      return "text-accent"
    }

    if (KEYWORDS.has(piece)) {
      return "text-primary"
    }

    if (next.startsWith("(")) {
      return "text-secondary-content"
    }

    if (piece.trim() === "" || /\w/.test(piece)) {
      return ""
    }

    return "text-base-content/70"
  }

  let pieces = $derived.by(() => {
    const raw = code.match(TOKEN) ?? []

    return raw.map((text, index) => ({
      text,
      kind: tone(text, raw[index + 1] ?? ""),
    }))
  })
</script>

<pre
  class={[
    "overflow-x-auto rounded-field bg-base-200",
    "px-4 py-3 text-sm leading-relaxed",
  ]}>{#each pieces as piece, index (index)}<span class={piece.kind}
      >{piece.text}</span
    >{/each}{#if caret}<span
      aria-hidden="true"
      class="caret inline-block h-4 w-2 bg-primary align-middle"
    ></span>{/if}</pre>

<style>
  .caret {
    animation: blink 1s steps(1) infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
</style>
