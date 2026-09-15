<script lang="ts">
  import { LineChart } from "layerchart"

  import { fade, slide } from "svelte/transition"

  import { Icon, calm, veil } from "@gpql/ui"
  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = {
    view: "table" | "chart"
    onblocked: () => void
  }

  let { view, onblocked }: Props = $props()

  type Msg = {
    payload: string
    qos: string
    retained: boolean
    at: number
  }

  let conn = $derived(workspace.active)

  let msgs = $derived.by(() => {
    const rows = workspace.browse.result?.rows ?? []

    return rows.map(
      (row): Msg => ({
        payload: row[0] ?? "",
        qos: row[1] ?? "0",
        retained: row[2] === "true",
        at: Number(row[3] ?? 0),
      }),
    )
  })

  let openAt = $state<number | null>(null)
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
    if (!conn || conn.draft.topic.trim() === "") {
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

{#if !workspace.browse.table}
  <p
    in:fade|local={veil()}
    class="py-10 text-center text-sm text-base-content/40"
  >
    {workspace.nouns.pick()}
  </p>
{:else if view === "chart"}
  <div in:fade|local={veil()} class="flex min-h-0 flex-1 flex-col">
    <div class="flex items-center gap-2 px-4 pb-2">
      <span class="text-xs text-base-content/40">{m.mqtt_field()}</span>

      <select
        bind:value={field}
        class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs
          outline-none"
      >
        {#each fields as name (name)}
          <option value={name}>{name === "" ? "payload" : name}</option>
        {/each}
      </select>
    </div>

    <div class="min-h-0 flex-1 px-4 pb-4">
      {#if points.length === 0}
        <p class="py-6 text-sm text-base-content/40">{m.mqtt_numeric_none()}</p>
      {:else}
        <LineChart data={points} x="at" y="amount" />
      {/if}
    </div>
  </div>
{:else if msgs.length === 0}
  <p
    in:fade|local={veil()}
    class="py-10 text-center text-sm text-base-content/40"
  >
    {workspace.nouns.emptyRows()}
  </p>
{:else}
  <div
    in:fade|local={veil()}
    class="min-h-0 flex-1 overflow-y-auto px-2 pb-2"
    style:scrollbar-gutter="stable"
  >
    {#each msgs as msg, index (index)}
      {@const open = openAt === index}

      <div
        class="mx-1 mb-1 rounded-field {open
          ? 'bg-base-200'
          : 'hover:bg-base-200/60'}"
      >
        <button
          type="button"
          onclick={() => (openAt = open ? null : index)}
          class="flex w-full items-center gap-2 px-2 py-1 text-left"
        >
          <span
            class="shrink-0 text-xs text-base-content/40"
            style="font-variant-numeric: tabular-nums">{clock(msg.at)}</span
          >

          <span
            class="shrink-0 rounded-selector bg-base-300 px-2 text-xs
              text-base-content/55">qos{msg.qos}</span
          >

          {#if msg.retained}
            <Icon
              icon="lucide:pin"
              class="size-3 shrink-0 text-base-content/40"
            />
          {/if}

          <span class="min-w-0 flex-1 truncate font-mono text-xs select-text">
            {msg.payload}
          </span>
        </button>

        {#if open}
          <div
            transition:slide|local={{ duration: calm() ? 0 : 140 }}
            class="px-2 pb-2"
          >
            <pre
              class="max-h-64 overflow-auto rounded-field bg-base-300 p-2
                font-mono text-xs select-text">{pretty(msg.payload)}</pre>

            <div class="flex items-center gap-2 pt-1">
              <span class="text-xs text-base-content/35">
                {new Date(msg.at).toLocaleString()}
              </span>

              <span class="flex-1"></span>

              <button
                type="button"
                onclick={() => navigator.clipboard.writeText(msg.payload)}
                class="rounded-selector bg-base-300 px-2 py-1 text-xs
                  hover:bg-base-100"
              >
                {m.menu_copy()}
              </button>
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

{#if conn}
  <footer class="shrink-0 border-t border-base-300/60 px-3 pt-2 pb-3">
    <div class="flex items-center gap-2 pb-1">
      <input
        bind:value={conn.draft.topic}
        placeholder={m.mqtt_topic()}
        spellcheck="false"
        class="min-w-0 flex-1 rounded-field bg-base-200 px-2 py-1 font-mono
          text-xs outline-none select-text placeholder:text-base-content/30"
      />

      <select
        bind:value={conn.draft.qos}
        aria-label="QoS"
        class="cursor-pointer rounded-field bg-base-200 px-2 py-1 text-xs
          outline-none"
      >
        <option value={0}>qos0</option>
        <option value={1}>qos1</option>
        <option value={2}>qos2</option>
      </select>

      <label
        class="flex cursor-pointer items-center gap-1 text-xs
          text-base-content/55"
      >
        <input
          type="checkbox"
          bind:checked={conn.draft.retain}
          class="size-3 accent-primary"
        />
        retain
      </label>

      <button
        type="button"
        onclick={send}
        disabled={sending || conn.draft.topic.trim() === ""}
        class="flex items-center gap-1 rounded-field bg-primary px-3 py-1
          text-xs text-primary-content disabled:opacity-40"
      >
        <Icon icon="lucide:send" class="size-3" />
        {m.mqtt_publish()}
      </button>
    </div>

    <textarea
      bind:value={conn.draft.payload}
      placeholder={m.mqtt_payload()}
      spellcheck="false"
      rows="2"
      onkeydown={event => {
        if (event.key === "Enter" && event.ctrlKey) {
          event.preventDefault()
          void send()
        }
      }}
      class="w-full resize-none rounded-field bg-base-200 px-2 py-1 font-mono
        text-xs outline-none select-text placeholder:text-base-content/30"
    ></textarea>

    {#if failure !== ""}
      <p
        transition:slide|local={{ duration: calm() ? 0 : 140 }}
        class="pt-1 text-xs text-error"
      >
        {failure}
      </p>
    {/if}
  </footer>
{/if}
