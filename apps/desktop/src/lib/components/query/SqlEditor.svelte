<script lang="ts">
  import { type Snippet, tick } from "svelte"

  import { Icon, Keycap, leave, rem, rise, type Ending } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { checkSql, lspComplete, run } from "$lib/session/commands"
  import { unitsAt } from "$lib/session/tokens"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Completion } from "$lib/types"

  import { formattable, tidy } from "./format"
  import { HEAVY, paint, type Span, shifted, tokenize } from "./paint"
  import "./tokens.css"

  type Props = {
    value: string
    selection: { start: number; end: number }
    onrun: () => void
    onclear: () => void
    find?: Snippet
  }

  let {
    value = $bindable(),
    selection = $bindable(),
    onrun,
    onclear,
    find,
  }: Props = $props()

  type Fault = { line: number; column: number; start: number; end: number }

  const LINE = 1.5
  const PAD = 0.5

  const DIALECTS: Record<string, string> = {
    sql: "SQL",
    cypher: "Cypher",
    flux: "Flux",
    s3: "S3",
    mqtt: "MQTT",
  }

  const EXAMPLES: Record<string, string> = {
    sql: "select * from",
    cypher: "match (n) return n limit 25",
    flux: 'from(bucket: "")',
    s3: "ls",
    mqtt: "publish sensors/room 21.5",
  }

  const uid = $props.id()
  const unit = rem(1)

  let input = $state<HTMLTextAreaElement | null>(null)
  let list = $state<HTMLUListElement | null>(null)
  let painted = $state<HTMLPreElement | null>(null)
  let colored = $state.raw<{ source: string; spans: Span[] }>({
    source: "",
    spans: [],
  })
  let hints = $state<Completion[]>([])
  let cursor = $state(0)
  let pending = $state(false)
  let forced = $state(false)
  let ending = $state<Ending>("cancel")
  let spot = $state({ left: 0, top: 0, above: false })
  let scroll = $state({ top: 0, left: 0 })
  let height = $state(0)
  let caret = $state(0)
  let focused = $state(false)
  let fault = $state<Fault | null>(null)
  let asked = 0

  let dialect = $derived(workspace.dialect)
  let heavy = $derived(value.length > HEAVY)
  let picked = $derived(selection.end > selection.start)
  let open = $derived(hints.length > 0 || (pending && forced))

  let starts = $derived.by(() => {
    const out = [0]
    let at = value.indexOf("\n")

    while (at !== -1) {
      out.push(at + 1)
      at = value.indexOf("\n", at + 1)
    }

    return out
  })

  let lines = $derived(starts.length)
  let row = $derived(lineOf(caret))
  let column = $derived(caret - starts[row] + 1)

  let chosenLines = $derived(
    picked
      ? lineOf(Math.max(selection.start, selection.end - 1)) -
          lineOf(selection.start) +
          1
      : 0,
  )

  let first = $derived(Math.max(0, Math.floor(scroll.top / unit / LINE) - 1))

  let last = $derived(
    Math.min(lines, first + Math.ceil(height / unit / LINE) + 3),
  )

  let numbers = $derived(
    Array.from(
      { length: Math.max(0, last - first) },
      (_, index) => first + index,
    ),
  )

  let from = $derived(starts[first] ?? 0)
  let to = $derived(last < lines ? starts[last] : value.length)

  let html = $derived.by(() => {
    const spans = heavy ? [] : shifted(value, colored)
    const shown = spans.flatMap(span =>
      span.end > from && span.start < to
        ? [
            {
              ...span,
              start: Math.max(span.start, from) - from,
              end: Math.min(span.end, to) - from,
            },
          ]
        : [],
    )

    const mark =
      picked && !focused
        ? { start: selection.start - from, end: selection.end - from }
        : null

    return `${paint(value.slice(from, to), shown, mark)} `
  })

  function lineOf(offset: number) {
    let low = 0
    let high = starts.length - 1

    while (low < high) {
      const middle = Math.ceil((low + high) / 2)

      if (starts[middle] <= offset) {
        low = middle
      } else {
        high = middle - 1
      }
    }

    return low
  }

  $effect(() => {
    const source = value
    const speech = dialect
    let live = true

    const timer = setTimeout(async () => {
      const spans = await tokenize(source, speech).catch(() => null)

      if (live && spans) {
        colored = { source, spans }
      }
    }, 24)

    return () => {
      live = false
      clearTimeout(timer)
    }
  })

  $effect(() => {
    const source = value
    const speech = dialect

    if (source.trim() === "" || source.length > HEAVY) {
      fault = null

      return
    }

    let live = true

    const timer = setTimeout(async () => {
      const found = await run(checkSql(source, speech)).catch(() => null)

      if (!live) {
        return
      }

      if (!found) {
        fault = null

        return
      }

      const start = unitsAt(source)[found.offset] ?? 0

      fault = {
        line: found.line + 1,
        column: start - (source.lastIndexOf("\n", start - 1) + 1) + 1,
        start,
        end: start + Math.max(1, found.text.length),
      }
    }, 400)

    return () => {
      live = false
      clearTimeout(timer)
    }
  })

  // replacing the text clamps scrollTop without a scroll event
  $effect(() => {
    void value

    void tick().then(() => {
      if (input) {
        scroll = { top: input.scrollTop, left: input.scrollLeft }
      }
    })
  })

  $effect(() => {
    list
      ?.querySelector(`[data-index="${cursor}"]`)
      ?.scrollIntoView({ block: "nearest" })
  })

  function wordBefore(at: number) {
    return value.slice(0, at).split(/\W/).pop() ?? ""
  }

  function dismiss(as: Ending = "cancel") {
    asked++
    ending = as
    hints = []
    pending = false
    forced = false
  }

  function place() {
    if (!input || !painted) {
      return
    }

    const walker = document.createTreeWalker(painted, NodeFilter.SHOW_TEXT)
    const target = input.selectionStart - from
    let seen = 0
    let node = walker.nextNode()

    while (node) {
      const length = node.textContent?.length ?? 0

      if (seen + length >= target) {
        break
      }

      seen += length
      node = walker.nextNode()
    }

    const range = document.createRange()

    if (node) {
      range.setStart(node, target - seen)
      range.collapse(true)
    } else {
      range.selectNodeContents(painted)
      range.collapse(false)
    }

    const box = range.getBoundingClientRect()
    const frame = input.getBoundingClientRect()
    const left = Math.min(Math.max(box.left, frame.left), frame.right)
    const above = window.innerHeight - box.bottom < rem(16)

    spot = {
      left: Math.min(left, window.innerWidth - rem(21)) / unit,
      top: (above ? window.innerHeight - box.top : box.bottom) / unit,
      above,
    }
  }

  async function suggest(force = false) {
    if (!input) {
      return
    }

    const ticket = ++asked
    const at = input.selectionStart
    const prefix = wordBefore(at)

    if (prefix === "" && !force && value[at - 1] !== ".") {
      dismiss()

      return
    }

    forced = force || forced
    pending = true
    await tick()
    place()

    const before = value.slice(0, at).split("\n")
    const line = before.length - 1
    const character = (before.at(-1) ?? "").length
    let found: Completion[] = []

    if (workspace.servers.includes(dialect)) {
      found = await run(lspComplete(dialect, value, line, character))
        .then(items =>
          items.filter(item =>
            item.label.toLowerCase().startsWith(prefix.toLowerCase()),
          ),
        )
        .catch(() => [])
    }

    if (ticket !== asked) {
      return
    }

    pending = false
    hints = (found.length > 0 ? found : workspace.suggest(prefix)).slice(0, 30)
    cursor = 0

    if (hints.length === 0) {
      forced = false
    }
  }

  function accept(item: Completion | undefined) {
    if (!item || !input) {
      return
    }

    const at = input.selectionStart
    const prefix = wordBefore(at)

    input.focus()
    input.setSelectionRange(at - prefix.length, at)
    document.execCommand("insertText", false, item.label)
    dismiss("confirm")
    track()
  }

  function track() {
    if (!input) {
      return
    }

    selection = { start: input.selectionStart, end: input.selectionEnd }
    caret =
      input.selectionDirection === "backward"
        ? input.selectionStart
        : input.selectionEnd
  }

  export function reveal(start: number, end: number) {
    if (!input) {
      return
    }

    input.focus()
    input.setSelectionRange(start, end)
    track()
  }

  export function focus() {
    input?.focus()
  }

  export async function format() {
    const partial = picked
    const start = partial ? selection.start : 0
    const end = partial ? selection.end : value.length
    const source = value.slice(start, end)

    if (!input || !formattable(dialect) || source.trim() === "") {
      return
    }

    let tidied: string

    try {
      tidied = await tidy(source, workspace.session?.kind ?? "")
    } catch (failure) {
      workspace.error = m.format_failed({
        reason: failure instanceof Error ? failure.message : String(failure),
      })

      return
    }

    if (tidied === source || value.slice(start, end) !== source) {
      return
    }

    input.focus()

    if (document.activeElement !== input) {
      return
    }

    input.setSelectionRange(start, end)
    document.execCommand("insertText", false, tidied)
    dismiss()

    if (partial) {
      input.setSelectionRange(start, start + tidied.length)
    }

    track()
  }

  function mirror() {
    if (!input || !painted) {
      return
    }

    scroll = { top: input.scrollTop, left: input.scrollLeft }

    if (open) {
      place()
    }
  }

  function clearAll() {
    if (!input) {
      return
    }

    input.focus()
    input.select()
    document.execCommand("delete")
    onclear()
    track()
  }

  function keys(event: KeyboardEvent) {
    if (event.isComposing) {
      return
    }

    if (event.key === " " && event.ctrlKey) {
      event.preventDefault()
      void suggest(true)

      return
    }

    if (event.code === "KeyF" && event.shiftKey && event.altKey) {
      event.preventDefault()
      dismiss()
      void format()

      return
    }

    if (open) {
      const moves: Record<string, number> = {
        ArrowDown: Math.min(cursor + 1, hints.length - 1),
        ArrowUp: Math.max(cursor - 1, 0),
        PageDown: Math.min(cursor + 8, hints.length - 1),
        PageUp: Math.max(cursor - 8, 0),
      }

      if (hints.length > 0 && event.key in moves) {
        event.preventDefault()
        cursor = moves[event.key]

        return
      }

      if (
        hints.length > 0 &&
        (event.key === "Tab" || (event.key === "Enter" && !event.ctrlKey))
      ) {
        event.preventDefault()
        accept(hints[cursor])

        return
      }

      if (event.key === "Escape") {
        event.preventDefault()
        event.stopPropagation()
        dismiss()

        return
      }
    }

    if (event.key === "Enter" && event.ctrlKey) {
      event.preventDefault()
      dismiss()
      onrun()

      return
    }

    if (event.key === "u" && event.ctrlKey) {
      event.preventDefault()
      clearAll()
    }
  }

  function typed(event: Event) {
    track()

    if (event instanceof InputEvent && event.isComposing) {
      return
    }

    void suggest()
  }

  function icon(kind: number) {
    if ([2, 3, 4].includes(kind)) {
      return "lucide:parentheses"
    }

    if ([5, 6, 10].includes(kind)) {
      return "lucide:columns-3"
    }

    if ([7, 8, 9, 22].includes(kind)) {
      return "lucide:table-2"
    }

    return kind === 14 ? "lucide:type" : "lucide:dot"
  }

  function jump() {
    const found = fault

    if (found) {
      void tick().then(() => reveal(found.start, found.end))
    }
  }
</script>

<div class="group/editor @container flex h-full min-h-0 flex-col">
  <div bind:clientHeight={height} class="relative flex min-h-0 flex-1">
    <div
      aria-hidden="true"
      class={[
        "relative w-12 shrink-0 overflow-hidden border-r",
        "border-base-content/10 text-right text-xs text-base-content/70",
        "tabular-nums select-none",
      ]}
    >
      <div style:transform="translateY({-scroll.top / unit}rem)">
        {#each numbers as index (index)}
          <span
            class={[
              "absolute right-3 leading-6",
              fault?.line === index + 1
                ? "text-error"
                : index === row && "group-focus-within/editor:text-primary",
            ]}
            style:top="{PAD + index * LINE}rem"
          >
            {index + 1}
          </span>
        {/each}
      </div>
    </div>

    <div class="relative min-w-0 flex-1">
      {#if !picked}
        <div
          aria-hidden="true"
          class={[
            "absolute inset-x-0 hidden h-6 bg-base-content/5",
            "group-focus-within/editor:block",
          ]}
          style:top="{PAD + row * LINE - scroll.top / unit}rem"
        ></div>
      {/if}

      <div aria-hidden="true" class="absolute inset-0 overflow-hidden">
        <pre
          bind:this={painted}
          class="sql-paint pointer-events-none"
          style:transform="translate({-scroll.left / unit}rem, {first * LINE -
            scroll.top / unit}rem)">{@html html}</pre>
      </div>

      <textarea
        bind:this={input}
        bind:value
        onkeydown={keys}
        onkeyup={track}
        oninput={typed}
        onpointerup={track}
        onselect={track}
        onscroll={mirror}
        onfocus={() => (focused = true)}
        onblur={() => {
          focused = false
          dismiss()
        }}
        wrap="off"
        spellcheck="false"
        autocapitalize="off"
        autocomplete="off"
        aria-label={m.editor_label()}
        aria-autocomplete="list"
        aria-controls={open ? `${uid}-list` : undefined}
        aria-activedescendant={hints.length > 0
          ? `${uid}-${cursor}`
          : undefined}
        placeholder={EXAMPLES[dialect] ?? EXAMPLES.sql}
        class="absolute inset-0 resize-none bg-transparent outline-none"
      ></textarea>
    </div>
  </div>

  <footer
    class={[
      "flex h-8 shrink-0 items-center gap-4 border-t border-base-content/10",
      "px-3 text-xs text-base-content/70",
    ]}
  >
    <span class="flex min-w-0 flex-1 items-center gap-2">
      {#if find}
        {@render find()}
      {:else if fault}
        <button
          type="button"
          onclick={jump}
          class={[
            "flex min-w-0 cursor-pointer items-center gap-2 text-left",
            "text-base-content transition-colors hover:text-primary",
          ]}
        >
          <Icon
            icon="lucide:triangle-alert"
            class="size-4 shrink-0 text-warning"
          />
          <span class="truncate">
            {m.editor_fault({ line: fault.line, column: fault.column })}
          </span>
        </button>
      {:else if picked}
        <Icon icon="lucide:text-select" class="size-4 shrink-0 text-primary" />
        <span class="truncate">
          {m.editor_selection({ lines: chosenLines })}
        </span>
      {:else}
        <span class="hidden items-center gap-2 @lg:flex">
          <Keycap keys={["ctrl", "enter"]} />
          {m.editor_run()}
        </span>

        <span class="hidden items-center gap-2 @2xl:flex">
          <Keycap keys={["ctrl", "space"]} />
          {m.key_complete()}
        </span>

        {#if formattable(dialect)}
          <span class="hidden items-center gap-2 @4xl:flex">
            <Keycap keys={["shift", "alt", "f"]} />
            {m.format_sql()}
          </span>
        {/if}
      {/if}
    </span>

    <span class="shrink-0 tabular-nums">
      {m.editor_position({ line: row + 1, column })}
    </span>

    <span class="hidden shrink-0 items-center gap-2 @md:flex">
      {DIALECTS[dialect] ?? dialect}

      {#if workspace.servers.includes(dialect)}
        <span class="badge badge-sm badge-soft badge-primary">LSP</span>
      {/if}
    </span>
  </footer>
</div>

{#if open}
  <div
    in:rise
    out:leave={{ as: ending }}
    class={[
      "floating lift fixed z-70 flex max-h-60 w-80 flex-col",
      spot.above ? "origin-bottom" : "origin-top",
    ]}
    style:left="{spot.left}rem"
    style:top={spot.above ? undefined : `${spot.top + 0.25}rem`}
    style:bottom={spot.above ? `${spot.top + 0.25}rem` : undefined}
  >
    {#if hints.length === 0}
      <p
        role="status"
        class="flex items-center gap-2 px-3 py-2 text-xs text-base-content/70"
      >
        <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
        {m.editor_completing()}
      </p>
    {:else}
      <ul
        bind:this={list}
        id="{uid}-list"
        role="listbox"
        aria-label={m.key_complete()}
        class="min-h-0 flex-1 overflow-y-auto p-1"
      >
        {#each hints as hint, index (index)}
          <li
            id="{uid}-{index}"
            role="option"
            aria-selected={index === cursor}
            data-index={index}
            onpointerdown={event => {
              event.preventDefault()
              accept(hint)
            }}
            onpointermove={() => (cursor = index)}
            class={[
              "relative flex cursor-pointer items-center gap-2 py-1 pr-2 pl-3",
              "text-sm",
              index === cursor ? "bg-primary/10" : "hover:bg-base-content/5",
            ]}
          >
            {#if index === cursor}
              <span
                aria-hidden="true"
                class="absolute inset-y-0 left-0 w-1 bg-primary"
              ></span>
            {/if}

            <Icon
              icon={icon(hint.kind)}
              class={[
                "size-4 shrink-0",
                index === cursor ? "text-primary" : "text-base-content/60",
              ]}
            />

            <span class="min-w-0 flex-1 truncate">{hint.label}</span>

            <span
              class={[
                "max-w-28 shrink-0 truncate text-xs",
                "text-base-content/70",
              ]}
            >
              {hint.detail}
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  pre,
  textarea {
    font: inherit;
    font-size: 0.875rem;
    font-kerning: none;
    font-variant-ligatures: none;
    line-height: 1.5rem;
    letter-spacing: 0;
    padding: 0.5rem 0.75rem;
    margin: 0;
    border: 0;
    tab-size: 2;
    white-space: pre;
  }

  pre {
    width: max-content;
    min-width: 100%;
    color: var(--color-base-content);
  }

  textarea {
    overflow: auto;
    color: transparent;
    caret-color: var(--color-primary);
  }

  textarea::placeholder {
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
  }

  textarea::selection,
  pre :global(.m) {
    background: color-mix(in oklch, var(--color-primary) 30%, transparent);
  }

  textarea::selection {
    color: transparent;
  }

  :global([data-theme="gpql"]) textarea::selection,
  :global([data-theme="gpql"]) pre :global(.m) {
    background: color-mix(in oklch, var(--color-primary) 20%, transparent);
  }
</style>
