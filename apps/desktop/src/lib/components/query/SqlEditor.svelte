<script lang="ts">
  import { Effect, Fiber } from "effect"

  import { highlightSql, lspComplete, run } from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Completion, SqlToken } from "$lib/types"

  type Props = {
    value: string
    selection: { start: number; end: number }
    onrun: () => void
    onclear: () => void
  }

  let {
    value = $bindable(),
    selection = $bindable(),
    onrun,
    onclear,
  }: Props = $props()

  let input = $state<HTMLTextAreaElement | null>(null)
  let painted = $state<HTMLPreElement | null>(null)
  let tokens = $state<SqlToken[]>([])
  let hints = $state<Completion[]>([])
  let cursor = $state(0)

  let painting: Fiber.RuntimeFiber<void, never> | null = null

  let pieces = $derived.by(() => {
    const out: { text: string; kind: string }[] = []
    let at = 0

    for (const token of tokens) {
      if (token.start < at || token.end > value.length) {
        continue
      }

      if (token.start > at) {
        out.push({ text: value.slice(at, token.start), kind: "" })
      }

      out.push({ text: value.slice(token.start, token.end), kind: token.kind })
      at = token.end
    }

    out.push({ text: value.slice(at), kind: "" })

    return out
  })

  $effect(() => {
    const source = value
    const dialect = workspace.dialect

    if (painting) {
      Effect.runFork(Fiber.interrupt(painting))
    }

    painting = Effect.runFork(
      Effect.gen(function* () {
        yield* Effect.sleep(24)

        tokens = yield* highlightSql(source, dialect)
      }).pipe(
        Effect.catchAll(() =>
          Effect.sync(() => {
            tokens = []
          }),
        ),
      ),
    )
  })

  function wordBefore(at: number) {
    const upto = value.slice(0, at)
    const match = upto.split(/[^\w.]/).pop() ?? ""

    return match
  }

  async function suggest(forced = false) {
    if (!input) {
      return
    }

    const at = input.selectionStart
    const prefix = wordBefore(at)

    // an empty prefix matches everything, which would put a completion list
    // under every space and let plain enter swallow the newline
    if (prefix === "" && !forced) {
      hints = []

      return
    }

    const dialect = workspace.dialect
    const before = value.slice(0, at).split("\n")
    const line = before.length - 1
    const character = (before.at(-1) ?? "").length

    if (workspace.servers.includes(dialect)) {
      try {
        const found = await run(lspComplete(dialect, value, line, character))

        if (found.length > 0) {
          hints = found
            .filter(item =>
              item.label.toLowerCase().startsWith(prefix.toLowerCase()),
            )
            .slice(0, 20)
          cursor = 0

          return
        }
      } catch {
        hints = []
      }
    }

    hints = workspace.suggest(prefix)
    cursor = 0
  }

  function accept(item: Completion | undefined) {
    if (!item || !input) {
      return
    }

    const at = input.selectionStart
    const prefix = wordBefore(at)
    const head = value.slice(0, at - prefix.length)

    value = head + item.label + value.slice(at)
    hints = []

    queueMicrotask(() => {
      const landing = head.length + item.label.length

      input?.setSelectionRange(landing, landing)
      input?.focus()
    })
  }

  function track() {
    if (!input) {
      return
    }

    selection = { start: input.selectionStart, end: input.selectionEnd }
  }

  // a jump made outside the editor, a find hit for instance, has to land here
  export function reveal(start: number, end: number) {
    if (!input) {
      return
    }

    input.focus()
    input.setSelectionRange(start, end)
    track()
  }

  function mirror() {
    if (input && painted) {
      painted.scrollTop = input.scrollTop
      painted.scrollLeft = input.scrollLeft
    }
  }

  function keys(event: KeyboardEvent) {
    if (event.key === " " && event.ctrlKey) {
      event.preventDefault()
      void suggest(true)

      return
    }

    if (hints.length > 0) {
      if (event.key === "ArrowDown") {
        event.preventDefault()
        cursor = Math.min(cursor + 1, hints.length - 1)

        return
      }

      if (event.key === "ArrowUp") {
        event.preventDefault()
        cursor = Math.max(cursor - 1, 0)

        return
      }

      if (event.key === "Tab" || (event.key === "Enter" && !event.ctrlKey)) {
        event.preventDefault()
        accept(hints[cursor])

        return
      }

      if (event.key === "Escape") {
        event.preventDefault()
        hints = []

        return
      }
    }

    if (event.key === "Enter" && event.ctrlKey) {
      event.preventDefault()
      onrun()
    }

    if (event.key === "u" && event.ctrlKey) {
      event.preventDefault()
      onclear()
    }

    if (event.key === " " && event.ctrlKey) {
      event.preventDefault()
      suggest(true)
    }
  }
</script>

<div class="relative h-24 font-mono text-sm">
  <pre
    bind:this={painted}
    aria-hidden="true"
    class="pointer-events-none absolute inset-0 overflow-hidden whitespace-pre-wrap
      break-words"><!--
 -->{#each pieces as piece, index (index)}<span class="tok-{piece.kind}"
        >{piece.text}</span
      >{/each}</pre>

  <textarea
    bind:this={input}
    bind:value
    onkeydown={keys}
    onkeyup={track}
    oninput={() => suggest()}
    onclick={track}
    onselect={track}
    onscroll={mirror}
    onblur={() => (hints = [])}
    spellcheck="false"
    placeholder="select * from"
    class="absolute inset-0 resize-none overflow-auto bg-transparent whitespace-pre-wrap
      break-words text-transparent caret-primary outline-none
      placeholder:text-base-content/25"
  ></textarea>

  {#if hints.length > 0}
    <ul
      class="absolute top-full left-0 z-40 mt-1 max-h-48 w-64 overflow-y-auto
        rounded-box floating p-1 lift"
    >
      {#each hints as hint, index (hint.label)}
        <li>
          <button
            type="button"
            onmousedown={event => {
              event.preventDefault()
              accept(hint)
            }}
            onmouseenter={() => (cursor = index)}
            class="flex w-full items-center gap-2 rounded-field px-2 py-1 text-left
              text-xs {index === cursor ? 'bg-primary/10 text-primary' : ''}"
          >
            <span class="min-w-0 flex-1 truncate">{hint.label}</span>

            <span class="shrink-0 text-base-content/35">{hint.detail}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  pre,
  textarea {
    font: inherit;
    line-height: 1.5;
    padding: 0;
    margin: 0;
    border: 0;
    tab-size: 2;
  }

  :global(.tok-keyword),
  :global(.tok-conditional),
  :global(.tok-storageclass) {
    color: var(--color-info);
  }

  :global(.tok-string) {
    color: var(--color-secondary-content);
  }

  :global(.tok-number),
  :global(.tok-float),
  :global(.tok-boolean) {
    color: var(--color-warning);
  }

  :global(.tok-function),
  :global(.tok-type) {
    color: var(--color-primary);
  }

  :global(.tok-comment) {
    color: color-mix(in oklch, currentColor 45%, transparent);
    font-style: italic;
  }

  :global(.tok-operator),
  :global(.tok-punctuation) {
    color: color-mix(in oklch, currentColor 60%, transparent);
  }

  :global(.tok-field),
  :global(.tok-attribute),
  :global(.tok-parameter),
  :global(.tok-property) {
    color: var(--color-accent);
  }
</style>
