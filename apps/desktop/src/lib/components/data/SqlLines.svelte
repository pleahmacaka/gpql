<script lang="ts">
  import { highlightSql, run } from "$lib/session/commands"
  import { splitTokens } from "$lib/session/tokens"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { SqlToken } from "$lib/types"

  import { TONES } from "$lib/components/query/paint"
  import "$lib/components/query/tokens.css"

  type Piece = { text: string; kind: string }

  type Props = { text: string; wrap?: boolean; class?: string }

  let { text, wrap = false, class: extra = "" }: Props = $props()

  let tokens = $state<SqlToken[]>([])

  $effect(() => {
    const source = text
    const dialect = workspace.dialect

    if (!source) {
      tokens = []

      return
    }

    let live = true

    run(highlightSql(source, dialect))
      .then(found => {
        if (live) {
          tokens = found
        }
      })
      .catch(() => {
        if (live) {
          tokens = []
        }
      })

    return () => {
      live = false
    }
  })

  let lines = $derived.by(() => {
    const out: Piece[][] = [[]]

    for (const piece of splitTokens(text, tokens)) {
      piece.text.split("\n").forEach((part, index) => {
        if (index > 0) {
          out.push([])
        }

        if (part !== "") {
          out[out.length - 1].push({ text: part, kind: piece.kind })
        }
      })
    }

    return out
  })
</script>

<ol
  class={[
    "sql-paint text-xs leading-6 select-text",
    !wrap && "min-w-max",
    extra,
  ]}
>
  {#each lines as line, index (index)}
    <li class="flex bg-inherit">
      <span
        aria-hidden="true"
        class={[
          "sticky left-0 w-12 shrink-0 bg-inherit pr-4 text-right",
          "text-base-content/70 tabular-nums select-none",
        ]}
      >
        {index + 1}
      </span>

      <code
        class={[
          "min-w-0 pr-6",
          wrap ? "whitespace-pre-wrap wrap-anywhere" : "whitespace-pre",
        ]}
        >{#each line as piece, at (at)}<span class={TONES[piece.kind]}
            >{piece.text}</span
          >{/each}</code
      >
    </li>
  {/each}
</ol>
