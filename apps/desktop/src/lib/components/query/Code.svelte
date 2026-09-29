<script lang="ts">
  import { workspace } from "$lib/session/workspace.svelte"

  import { paint, type Span, shifted, tokenize } from "./paint"
  import "./tokens.css"

  type Props = { code: string; class?: string }

  let { code, class: extra = "" }: Props = $props()

  let colored = $state.raw<{ source: string; spans: Span[] }>({
    source: "",
    spans: [],
  })

  let html = $derived(paint(code, shifted(code, colored)))

  $effect(() => {
    const source = code
    let live = true

    tokenize(source, workspace.dialect)
      .then(spans => {
        if (live) {
          colored = { source, spans }
        }
      })
      .catch(() => undefined)

    return () => {
      live = false
    }
  })
</script>

<pre
  class={[
    "sql-paint text-sm leading-6 whitespace-pre-wrap wrap-anywhere select-text",
    extra,
  ]}>{@html html}</pre>
