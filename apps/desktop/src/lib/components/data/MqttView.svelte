<script lang="ts">
  import { LineChart } from "layerchart"

  import { fade, slide } from "svelte/transition"

  import {
    Dropdown,
    EmptyState,
    Icon,
    Keycap,
    Segmented,
    TIMING,
    tooltip,
    veil,
  } from "@gpql/ui"
  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = {
    view: "table" | "chart"
    onblocked: () => void
  }

  let { view, onblocked }: Props = $props()

  type Msg = {
    id: string
    payload: string
    qos: string
    retained: boolean
    at: number
  }

  const LEVELS = [
    { value: "0", label: "0" },
    { value: "1", label: "1" },
    { value: "2", label: "2" },
  ]

  let conn = $derived(workspace.active)

  let msgs = $derived.by(() => {
    const rows = workspace.browse.result?.rows ?? []
    const seen = new Map<number, number>()
    const out: Msg[] = []

    for (let index = rows.length - 1; index >= 0; index--) {
      const row = rows[index]
      const at = Number(row[3] ?? 0)
      const twin = seen.get(at) ?? 0

      seen.set(at, twin + 1)
      out.push({
        id: `${at}:${twin}`,
        payload: row[0] ?? "",
        qos: row[1] ?? "0",
        retained: row[2] === "true",
        at,
      })
    }

    return out.reverse()
  })

  let openId = $state<string | null>(null)
  let field = $state("")

  let followed = $state("")

  $effect(() => {
    const table = workspace.browse.table

    if (conn && table && table !== followed) {
      followed = table
      conn.draft = { ...conn.draft, topic: table }
    }
  })

  const pad = (value: number, wide = 2) => String(value).padStart(wide, "0")

  function clock(at: number) {
    const when = new Date(at)

    return `${pad(when.getHours())}:${pad(when.getMinutes())}:${pad(
      when.getSeconds(),
    )}.${pad(when.getMilliseconds(), 3)}`
  }

  function pretty(payload: string) {
    try {
      return JSON.stringify(JSON.parse(payload), null, 2)
    } catch {
      return payload
    }
  }

  function numbers(payload: string): Record<string, number> {
    const raw = Number(payload)

    if (payload.trim() !== "" && !Number.isNaN(raw)) {
      return { "": raw }
    }

    try {
      const parsed: unknown = JSON.parse(payload)
      const out: Record<string, number> = {}

      const walk = (held: unknown, path: string) => {
        if (typeof held === "number" && Number.isFinite(held)) {
          out[path] = held
        } else if (held && typeof held === "object" && !Array.isArray(held)) {
          for (const [key, child] of Object.entries(held)) {
            walk(child, path === "" ? key : `${path}.${key}`)
          }
        }
      }

      walk(parsed, "")

      return out
    } catch {
      return {}
    }
  }

  let fields = $derived.by(() => {
    const found = new Set<string>()

    for (const msg of msgs.slice(0, 50)) {
      for (const key of Object.keys(numbers(msg.payload))) {
        found.add(key)
      }
    }

    return [...found].sort()
  })

  let picked = $derived(fields.includes(field) ? field : (fields[0] ?? ""))

  let choices = $derived(
    fields.map(name => ({
      value: name,
      label: name === "" ? m.mqtt_payload() : name,
    })),
  )

  let points = $derived.by(() => {
    const out: { at: Date; amount: number }[] = []

    for (const msg of msgs) {
      const amount = numbers(msg.payload)[picked]

      if (amount !== undefined) {
        out.push({ at: new Date(msg.at), amount })
      }
    }

    return out.sort((a, b) => a.at.getTime() - b.at.getTime()).slice(-200)
  })

  let sending = $state(false)
  let failure = $state("")

  async function send() {
    if (!conn || conn.draft.topic.trim() === "" || sending) {
      return
    }

    if (workspace.readOnly) {
      onblocked()

      return
    }

    sending = true
    failure = ""

    try {
      await conn.mqttPublish(
        conn.draft.topic.trim(),
        conn.draft.payload,
        conn.draft.qos,
        conn.draft.retain,
      )
      conn.draft = { ...conn.draft, payload: "" }
    } catch (error) {
      failure = String(error)
    } finally {
      sending = false
    }
  }
</script>

<div class="relative min-h-0 flex-1">
  {#if !workspace.browse.table}
    <div
      in:fade|local={veil()}
      class="absolute inset-0 grid place-items-center"
    >
      <EmptyState art="sheet" title={workspace.nouns.pick()} />
    </div>
  {:else if view === "chart"}
    <div
      in:fade|local={veil()}
      class="absolute inset-0 flex flex-col gap-3 px-4 pt-3 pb-4"
    >
      <div class="flex items-center gap-2 text-xs">
        <span class="text-base-content/70">{m.mqtt_field()}</span>

        {#if choices.length > 0}
          <Dropdown
            small
            label={m.mqtt_field()}
            value={picked}
            options={choices}
            onpick={next => (field = next)}
          />
        {/if}
      </div>

      <div class="relative min-h-0 flex-1">
        {#if points.length === 0}
          <div class="absolute inset-0 grid place-items-center">
            <EmptyState art="graph" title={m.mqtt_numeric_none()} />
          </div>
        {:else}
          <LineChart data={points} x="at" y="amount" />
        {/if}
      </div>
    </div>
  {:else if msgs.length === 0}
    <div
      in:fade|local={veil()}
      class="absolute inset-0 grid place-items-center"
    >
      <EmptyState art="mark" title={workspace.nouns.emptyRows()} />
    </div>
  {:else}
    <ol
      in:fade|local={veil()}
      class="absolute inset-0 overflow-y-auto"
      style:scrollbar-gutter="stable"
    >
      {#each msgs as msg (msg.id)}
        {@const open = openId === msg.id}

        <li
          class={[
            "border-b border-base-content/5",
            open ? "bg-base-content/5" : "hover:bg-base-content/5",
          ]}
        >
          <button
            type="button"
            aria-expanded={open}
            onclick={() => (openId = open ? null : msg.id)}
            class={[
              "flex h-9 w-full cursor-pointer items-center gap-3 px-4",
              "text-left text-sm outline-offset-0",
            ]}
          >
            <Icon
              icon="lucide:chevron-right"
              class={[
                "size-3 shrink-0 text-base-content/60 transition-transform",
                open && "rotate-90",
              ]}
            />

            <span class="shrink-0 text-xs text-base-content/70 tabular-nums">
              {clock(msg.at)}
            </span>

            <span class="badge badge-xs badge-soft shrink-0 tabular-nums">
              {m.mqtt_qos({ level: msg.qos })}
            </span>

            {#if msg.retained}
              <span class="badge badge-xs badge-soft badge-info shrink-0">
                {m.mqtt_retain()}
              </span>
            {/if}

            <span class="min-w-0 flex-1 truncate">{msg.payload}</span>
          </button>

          {#if open}
            <div
              transition:slide|local={{ duration: TIMING.quick }}
              class="flex flex-col gap-2 px-4 pb-3 pl-10"
            >
              <pre
                class={[
                  "max-h-64 overflow-auto bg-base-200 p-3 text-xs leading-5",
                  "whitespace-pre-wrap wrap-anywhere select-text hairline",
                ]}>{pretty(msg.payload)}</pre>

              <div class="flex items-center gap-2">
                <span class="flex-1 text-xs text-base-content/70 tabular-nums">
                  {new Date(msg.at).toLocaleString(workspace.locale)}
                </span>

                <button
                  type="button"
                  onclick={() => navigator.clipboard.writeText(msg.payload)}
                  class="btn btn-soft btn-sm"
                >
                  <Icon icon="lucide:copy" class="size-4" />
                  {m.menu_copy()}
                </button>
              </div>
            </div>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</div>

{#if conn}
  <footer
    class="flex shrink-0 flex-col gap-2 border-t border-base-content/10 p-3"
  >
    <div class="flex items-center gap-2">
      <label class="input input-sm min-w-0 flex-1 bg-base-100">
        <Icon icon="lucide:send" class="size-4 shrink-0 text-base-content/60" />

        <input
          bind:value={conn.draft.topic}
          placeholder={m.mqtt_topic()}
          aria-label={m.mqtt_topic()}
          spellcheck="false"
          class="min-w-0 grow select-text placeholder:text-base-content/60"
        />
      </label>

      <Segmented
        small
        label={m.mqtt_qos_label()}
        options={LEVELS}
        value={String(conn.draft.qos)}
        onpick={next => {
          if (conn) {
            conn.draft = { ...conn.draft, qos: Number(next) }
          }
        }}
      />

      <label
        class={[
          "flex shrink-0 cursor-pointer items-center gap-2 text-xs",
          "text-base-content/70",
        ]}
      >
        <input
          type="checkbox"
          bind:checked={conn.draft.retain}
          class="toggle toggle-sm toggle-primary"
        />
        {m.mqtt_retain()}
      </label>
    </div>

    <textarea
      bind:value={conn.draft.payload}
      placeholder={m.mqtt_payload()}
      aria-label={m.mqtt_payload()}
      spellcheck="false"
      rows="2"
      onkeydown={event => {
        if (event.key === "Enter" && event.ctrlKey && !event.isComposing) {
          event.preventDefault()
          void send()
        }
      }}
      class={[
        "textarea w-full resize-none bg-base-100 text-sm select-text",
        "placeholder:text-base-content/60",
      ]}
    ></textarea>

    <div class="flex h-8 items-center gap-3">
      {#if failure !== ""}
        <p
          in:fade|local={veil()}
          role="alert"
          use:tooltip={failure}
          class="flex min-w-0 flex-1 items-center gap-2 text-xs text-error"
        >
          <Icon icon="lucide:circle-alert" class="size-4 shrink-0" />
          <span class="truncate select-text">{failure}</span>
        </p>
      {:else}
        <Keycap keys={["ctrl", "enter"]} class="flex-1" />
      {/if}

      <button
        type="button"
        onclick={send}
        disabled={sending || conn.draft.topic.trim() === ""}
        class="btn btn-primary btn-sm font-medium"
      >
        <Icon
          icon={sending ? "lucide:loader-circle" : "lucide:send"}
          class={["size-4", sending && "animate-spin"]}
        />
        {m.mqtt_publish()}
      </button>
    </div>
  </footer>
{/if}
