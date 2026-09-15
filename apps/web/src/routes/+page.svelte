<script lang="ts">
  import {
    DataGrid,
    Icon,
    Keycap,
    Logo,
    ResultChart,
    SchemaBoard,
    SessionCard,
  } from "@gpql/ui"
  import { SvelteFlowProvider } from "@xyflow/svelte"
  import { LineChart } from "layerchart"

  import DemoWindow from "$lib/components/marketing/DemoWindow.svelte"
  import Sql from "$lib/components/marketing/Sql.svelte"
  import * as sample from "$lib/components/marketing/sample"
  import { reveal } from "$lib/reveal"

  let draft = $state({ ...sample.draft })
  let readOnly = $state(true)
  let dataView = $state<"table" | "chart">("table")

  const SUMMARY =
    "A SQL client that keeps everything on your disk. Fifteen databases, one window, read only until you say otherwise."

  let mqttView = $state<"feed" | "chart">("feed")
  let mqttSubs = $state([...sample.mqttSubs])
  let mqttFilter = $state("")
  let mqttShut = $state<Record<string, boolean>>({})
  let mqttTopic = $state("sensors/temperature")
  let mqttFeed = $state([...sample.mqttFeed])
  let mqttOpen = $state<number | null>(2)
  let mqttDraftTopic = $state("sensors/temperature")
  let mqttPayload = $state('{"temp":21.5,"unit":"C","node":"attic"}')
  let mqttQos = $state(0)
  let mqttRetain = $state(false)

  let mqttTree = $derived.by(() => {
    const shut = (path: string) => mqttShut[path] ?? path.startsWith("$SYS")

    return sample.mqttTopics.filter(node => {
      const slash = node.path.lastIndexOf("/")
      const parent = slash === -1 ? null : node.path.slice(0, slash)

      return parent === null || !shut(parent)
    })
  })

  const pad = (value: number, wide = 2) => String(value).padStart(wide, "0")

  function clock(at: Date) {
    return `${pad(at.getHours())}:${pad(at.getMinutes())}:${pad(
      at.getSeconds(),
    )}.${pad(at.getMilliseconds(), 3)}`
  }

  function pretty(payload: string) {
    try {
      return JSON.stringify(JSON.parse(payload), null, 2)
    } catch {
      return payload
    }
  }

  function addSub() {
    const next = mqttFilter.trim()

    if (next === "" || mqttSubs.some(sub => sub.filter === next)) {
      return
    }

    mqttSubs = [...mqttSubs, { filter: next, qos: 1 }]
    mqttFilter = ""
  }

  function pickTopic(path: string) {
    mqttTopic = path
    mqttDraftTopic = path
  }

  function publish() {
    if (mqttDraftTopic.trim() === "" || mqttPayload.trim() === "") {
      return
    }

    mqttFeed = [
      {
        at: clock(new Date()),
        qos: String(mqttQos),
        retained: mqttRetain,
        payload: mqttPayload,
      },
      ...mqttFeed,
    ]

    mqttPayload = ""
    mqttOpen = 0
  }

  type Card = { icon: string; title: string; text: string; keys?: string[] }

  const machinery: Card[] = [
    {
      icon: "lucide:command",
      title: "Ctrl+K opens everything",
      text: "One palette over tables, saved queries, recents and toggles. Pick a saved query and it lands back in the editor it left.",
      keys: ["ctrl", "k"],
    },
    {
      icon: "lucide:radar",
      title: "What is running gets found",
      text: "The scan reads listening ports and docker containers for PostgreSQL, MySQL, Redis, ClickHouse, InfluxDB, Neo4j, GreptimeDB and MQTT brokers. One click connects, and the login is remembered for next time.",
    },
    {
      icon: "lucide:file-code-2",
      title: "Every object shows its source",
      text: "A view, index, trigger, routine, sequence or type opens the statement that made it, on PostgreSQL, MySQL, SQLite and DuckDB. Highlighted, read only, one click to copy.",
    },
    {
      icon: "lucide:filter",
      title: "Filters stay on the column",
      text: "Click a header and an operator plus a value drops in. Active filters ride under the grid as chips, and an empty result still keeps them visible.",
    },
  ]
</script>

<svelte:head>
  <title>GPQL</title>
  <meta name="description" content={SUMMARY} />
</svelte:head>

<div class="relative">
  <div
    aria-hidden="true"
    class="pointer-events-none absolute inset-x-0 top-0 h-192 grain"
  ></div>

  <div class="relative mx-auto max-w-5xl px-4 pb-24 sm:px-6">
    <div class="sticky top-0 z-40 -mx-4 px-4 pt-2 pb-3 sm:-mx-6 sm:px-6">
      <nav
        class="flex items-center gap-1 rounded-box bg-base-100/80 p-2 pl-3
          text-sm backdrop-blur-lg lift"
      >
        <a
          href="/"
          class="flex items-center gap-2 pr-3 font-display text-base
            font-medium"
        >
          <Logo class="size-5" />
          GPQL
        </a>

        {#each [{ href: "#engines", label: "Databases" }, { href: "#screens", label: "Screens" }, { href: "#mqtt", label: "MQTT" }, { href: "#sync", label: "Sync" }] as link (link.href)}
          <a
            href={link.href}
            class="hidden rounded-field px-3 py-2 text-base-content/60
              transition-colors hover:bg-base-200 hover:text-base-content
              sm:block"
          >
            {link.label}
          </a>
        {/each}

        <span class="flex-1"></span>

        <a
          href="/download"
          class="hidden rounded-field px-3 py-2 text-base-content/60
            transition-colors hover:bg-base-200 hover:text-base-content sm:block"
        >
          Download
        </a>

        <a
          href="/account"
          class="rounded-field bg-base-200 px-3 py-2 transition-colors
            hairline hover:bg-base-300"
        >
          Account
        </a>
      </nav>
    </div>

    <header
      class="grid items-center gap-8 pt-6 pb-12 sm:gap-10 sm:pt-8 sm:pb-14
        lg:grid-cols-2"
    >
      <div data-reveal use:reveal>
        <h1
          class="pt-4 font-display text-4xl leading-tight font-bold tracking-tight
            sm:text-5xl"
        >
          Open quickly.<br />
          Just works.<br />
          <span class="text-base-content/40">Keep it safe.</span>
        </h1>

        <p class="max-w-md pt-5 text-base-content/65">{SUMMARY}</p>

        <div class="flex flex-wrap items-center gap-3 pt-7">
          <a
            href="https://github.com/pleahmacaka/gpql/releases/latest"
            class="flex items-center gap-2 rounded-field bg-primary px-4 py-3
              text-sm text-primary-content transition-colors hover:bg-primary/90"
          >
            <Icon icon="lucide:download" class="size-4" />
            Download for Windows
          </a>

          <a
            href="#engines"
            class="rounded-field bg-base-100 px-4 py-3 text-sm hairline
              transition-colors hover:bg-base-300"
          >
            See the drivers
          </a>
        </div>
      </div>

      <div class="relative min-w-0" data-reveal use:reveal>
        <div
          aria-hidden="true"
          class="absolute -inset-4 rounded-box bg-primary/10 blur-3xl
            sm:-inset-6"
        ></div>

        <DemoWindow chip="no session" chipIcon="lucide:plus">
          <div class="bg-base-200 px-4 py-6 sm:px-8">
            <section
              class="w-96 max-w-full rounded-box bg-base-100 p-4 lift sm:mx-auto"
            >
              <h2 class="mb-3 text-sm font-medium">New session</h2>

              <SessionCard
                bind:draft
                backends={sample.backends}
                probe={{ tone: "good", text: "PostgreSQL 18.4 answered in 6 ms" }}
                {readOnly}
                ontoggleReadOnly={() => (readOnly = !readOnly)}
              />
            </section>
          </div>
        </DemoWindow>
      </div>
    </header>

    <section id="engines" class="pt-10" data-reveal use:reveal>
      <h2
        class="pt-3 font-display text-2xl font-bold tracking-tight sm:text-3xl"
      >
        Every database speaks for itself.
      </h2>

      <p class="max-w-2xl pt-4 text-base-content/65">
        Every backend goes through the driver its own people maintain. No
        wrapper protocol, no hand-rolled HTTP.
      </p>

      <ul class="grid grid-cols-2 gap-2 pt-8 sm:grid-cols-3 lg:grid-cols-4">
        {#each sample.engines as engine (engine.name)}
          <li
            class="flex items-center gap-3 rounded-field bg-base-100 px-3 py-3
              hairline transition-colors hover:bg-base-300"
          >
            <Icon icon={engine.icon} class="size-4 shrink-0" />

            <span class="min-w-0 flex-1">
              <span class="flex items-center gap-2">
                <span class="truncate text-sm">{engine.name}</span>

                {#if engine.wip}
                  <span
                    class="shrink-0 rounded-selector bg-base-300 px-2 font-mono
                      text-xs text-base-content/45"
                  >
                    wip
                  </span>
                {/if}
              </span>

              <span class="block font-mono text-xs text-base-content/40">
                {engine.note}
              </span>
            </span>
          </li>
        {/each}
      </ul>
    </section>

    <div class="mt-16 h-px rule"></div>

    <section id="screens" class="pt-12 sm:pt-16">
      <div data-reveal use:reveal>
        <h2
          class="pt-3 font-display text-2xl font-bold tracking-tight sm:text-3xl"
        >
          Three screens. No fourth one.
        </h2>
      </div>

      <div class="space-y-12 pt-8">
        <article data-reveal use:reveal>
          <div class="flex flex-wrap items-baseline gap-3 pb-3">
            <h3 class="font-display text-lg font-medium">Data</h3>

            <p class="text-sm text-base-content/55">
              Sorting and filtering are pushed down to the server, so the
              answer covers the whole table and not the page you happen to
              have loaded. Export what you are looking at, filters and all.
            </p>
          </div>

          <div
            class="rounded-box bg-linear-to-b from-base-100 to-base-200 p-2
              ring-1 ring-base-content/5 sm:p-3"
          >
            <DemoWindow chip="roomy" tab="Data">
            <div class="flex h-96 gap-2 bg-base-200 p-2">
              <aside class="w-52 shrink-0 rounded-box bg-base-100 p-2 lift">
                {#each sample.tables as table (table.name)}
                  <div
                    class="flex items-center gap-2 rounded-field px-2 py-2
                      text-sm {table.name === 'message'
                      ? 'bg-primary/10 text-primary'
                      : ''}"
                  >
                    <Icon icon="lucide:table-2" class="size-4 opacity-60" />
                    <span class="flex-1 truncate">{table.name}</span>
                    <span class="text-xs text-base-content/40">{table.rows}</span>
                  </div>
                {/each}
              </aside>

              <section
                class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift"
              >
                <header class="flex items-baseline gap-2 px-4 pt-2 pb-1">
                  <h4 class="text-sm font-medium">message</h4>

                  <span class="text-xs text-base-content/45">
                    {sample.columns.length} columns
                  </span>

                  <span class="flex-1"></span>

                  <div
                    class="flex gap-1 self-center rounded-selector bg-base-200
                      p-1"
                  >
                    {#each [{ id: "table", icon: "lucide:table-2" }, { id: "chart", icon: "lucide:bar-chart-3" }] as option (option.id)}
                      <button
                        type="button"
                        aria-label={option.id}
                        onclick={() =>
                          (dataView = option.id as "table" | "chart")}
                        class="rounded-selector px-2 py-1 transition-colors
                          {dataView === option.id
                          ? 'bg-base-100 hairline'
                          : 'text-base-content/45'}"
                      >
                        <Icon icon={option.icon} class="size-4" />
                      </button>
                    {/each}
                  </div>
                </header>

                {#if dataView === "chart"}
                  <ResultChart columns={sample.columns} rows={sample.rows} />
                {:else}
                  <DataGrid columns={sample.columns} rows={sample.rows} />
                {/if}
              </section>
            </div>
            </DemoWindow>
          </div>
        </article>

        <article data-reveal use:reveal>
          <div class="flex flex-wrap items-baseline gap-3 pb-3">
            <h3 class="font-display text-lg font-medium">Query</h3>

            <p class="text-sm text-base-content/55">
              Tree-sitter highlighting per dialect, completion from a real
              language server, and an ask bar that writes the SQL with your own
              key. When something drags, read the plan; a model will read it
              with you and say what it would add.
            </p>
          </div>

          <div
            class="rounded-box bg-linear-to-b from-base-100 to-base-200 p-2
              ring-1 ring-base-content/5 sm:p-3"
          >
            <DemoWindow chip="roomy" tab="Query">
            <div class="h-96 bg-base-200 p-2">
              <div
                class="flex h-full flex-col gap-2 rounded-box bg-base-100 p-4
                  lift"
              >
                <div
                  class="flex items-center gap-2 rounded-field bg-base-200 px-3
                    py-2"
                >
                  <Icon icon="lucide:sparkles" class="size-4 text-accent" />

                  <span class="flex-1 text-sm text-base-content/70">
                    {sample.ask[0].text}
                  </span>
                </div>

                <Sql code={sample.ask[1].text} />

                <p
                  class="flex items-center gap-2 font-mono text-xs
                    text-base-content/45"
                >
                  <Icon icon="lucide:play" class="size-3" />
                  3 rows in 4 ms, read only
                </p>
              </div>
            </div>
            </DemoWindow>
          </div>
        </article>

        <article data-reveal use:reveal>
          <div class="flex flex-wrap items-baseline gap-3 pb-3">
            <h3 class="font-display text-lg font-medium">Schema</h3>

            <p class="text-sm text-base-content/55">
              Tables laid out by what they point at, walkable with the arrow
              keys, and grouped by hand or, if you turn it on, by a model.
              Views, indexes, routines and triggers sit beside them, and any of
              them will show you the statement that made it.
            </p>
          </div>

          <div
            class="rounded-box bg-linear-to-b from-base-100 to-base-200 p-2
              ring-1 ring-base-content/5 sm:p-3"
          >
            <DemoWindow chip="roomy" tab="Schema">
            <div class="h-96 bg-base-200 p-2">
              <div class="h-full overflow-hidden rounded-box bg-base-100 lift">
                <SvelteFlowProvider>
                  <SchemaBoard tables={sample.tables} keyboard={false} />
                </SvelteFlowProvider>
              </div>
            </div>
            </DemoWindow>
          </div>
        </article>
      </div>
    </section>

    <section id="mqtt" class="pt-16 sm:pt-20">
      <div data-reveal use:reveal>
        <h2
          class="pt-3 font-display text-2xl font-bold tracking-tight sm:text-3xl"
        >
          Topics, not tables.
        </h2>

        <p class="max-w-2xl pt-4 text-base-content/65">
          Connect to a broker and the same window speaks a different language:
          the sidebar becomes a topic tree, grouped by the slash in each path,
          a running count on every leaf. Subscription filters are added and
          dropped while the feed keeps moving.
        </p>

        <p class="max-w-2xl pt-3 text-base-content/65">
          Messages land newest first, stamped to the millisecond, with their
          QoS and retained flags showing. A click unfolds the payload into
          readable JSON, a numeric one charts itself over time, and the
          composer publishes straight back with the QoS and retain you pick.
          NanoMQ, EMQX and Mosquitto are found by the same scan that finds
          everything else.
        </p>
      </div>

      <div class="pt-8" data-reveal use:reveal>
        <div
          class="rounded-box bg-linear-to-b from-base-100 to-base-200 p-2
            ring-1 ring-base-content/5 sm:p-3"
        >
          <DemoWindow chip="emqx" chipIcon="simple-icons:mqtt" tab="Data">
          <div class="flex h-96 gap-2 bg-base-200 p-2">
            <aside
              class="flex w-52 shrink-0 flex-col overflow-hidden rounded-box
                bg-base-100 p-2 lift"
            >
              <p class="px-1 pt-1 text-xs text-base-content/40">Subscriptions</p>

              <div class="flex flex-wrap gap-1 px-1 pt-1">
                {#each mqttSubs as sub (sub.filter)}
                  <span
                    class="flex items-center gap-1 rounded-selector bg-base-200
                      px-2 py-1 text-xs"
                  >
                    {sub.filter}
                    <span class="text-base-content/35">qos{sub.qos}</span>

                    <button
                      type="button"
                      aria-label="drop {sub.filter}"
                      onclick={() =>
                        (mqttSubs = mqttSubs.filter(entry => entry !== sub))}
                      class="text-base-content/35 hover:text-base-content"
                    >
                      <Icon icon="lucide:x" class="size-3" />
                    </button>
                  </span>
                {/each}
              </div>

              <div class="flex gap-1 px-1 pt-2">
                <input
                  bind:value={mqttFilter}
                  placeholder="ex. sensors/#"
                  onkeydown={event => event.key === "Enter" && addSub()}
                  class="min-w-0 flex-1 rounded-field bg-base-200 px-2 py-1
                    text-xs outline-none select-text
                    placeholder:text-base-content/30"
                />

                <button
                  type="button"
                  aria-label="add filter"
                  onclick={addSub}
                  class="rounded-field bg-base-200 px-2 py-1 hover:bg-base-300"
                >
                  <Icon icon="lucide:plus" class="size-4" />
                </button>
              </div>

              <div class="min-h-0 flex-1 overflow-y-auto pt-2">
                {#each mqttTree as node (node.path)}
                  {#if node.folder}
                    {@const shut =
                      mqttShut[node.path] ?? node.path.startsWith("$SYS")}

                    <div class="flex items-center rounded-field hover:bg-base-200">
                      <button
                        type="button"
                        aria-expanded={!shut}
                        onclick={() =>
                          (mqttShut = { ...mqttShut, [node.path]: !shut })}
                        class="flex min-w-0 flex-1 items-center gap-2 px-2 py-2
                          text-left"
                        style:margin-left="{node.depth * 0.75}rem"
                      >
                        <Icon
                          icon={shut
                            ? "lucide:chevron-right"
                            : "lucide:chevron-down"}
                          class="size-3 shrink-0 opacity-60"
                        />

                        <span
                          class="truncate text-sm text-base-content/60"
                          title={node.path}
                        >
                          {node.label}
                        </span>

                        <span class="shrink-0 text-xs text-base-content/30">
                          {node.inside}
                        </span>
                      </button>
                    </div>
                  {:else}
                    <div
                      class="flex items-center rounded-field {mqttTopic ===
                      node.path
                        ? 'bg-primary/10 text-primary'
                        : 'hover:bg-base-200'}"
                    >
                      <button
                        type="button"
                        onclick={() => pickTopic(node.path)}
                        aria-pressed={mqttTopic === node.path}
                        class="flex min-w-0 flex-1 items-center gap-2 px-2 py-2
                          text-left"
                        style:margin-left="{node.depth * 0.75 + 0.75}rem"
                      >
                        <Icon
                          icon="lucide:radio"
                          class="size-4 shrink-0 opacity-60"
                        />

                        <span class="truncate text-sm" title={node.path}>
                          {node.label}
                        </span>

                        {#if node.count > 0}
                          <span class="shrink-0 text-xs text-base-content/35">
                            {node.count}
                          </span>
                        {/if}
                      </button>
                    </div>
                  {/if}
                {/each}
              </div>
            </aside>

            <section
              class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift"
            >
              <header class="flex items-baseline gap-2 px-4 pt-2 pb-1">
                <h4 class="text-sm font-medium">{mqttTopic}</h4>

                <span class="text-xs text-base-content/45">
                  {mqttFeed.length} messages
                </span>

                <span class="flex-1"></span>

                <div
                  class="flex gap-1 self-center rounded-selector bg-base-200
                    p-1"
                >
                  {#each [{ id: "feed", icon: "lucide:list" }, { id: "chart", icon: "lucide:chart-line" }] as option (option.id)}
                    <button
                      type="button"
                      aria-label={option.id}
                      onclick={() =>
                        (mqttView = option.id as "feed" | "chart")}
                      class="rounded-selector px-2 py-1 transition-colors
                        {mqttView === option.id
                        ? 'bg-base-100 hairline'
                        : 'text-base-content/45'}"
                    >
                      <Icon icon={option.icon} class="size-4" />
                    </button>
                  {/each}
                </div>
              </header>

              {#if mqttView === "chart"}
                <div class="flex min-h-0 flex-1 flex-col">
                  <div class="flex items-center gap-2 px-4 pb-2">
                    <span class="text-xs text-base-content/40">field</span>

                    <span
                      class="rounded-field bg-base-200 px-2 py-1 text-xs"
                    >
                      temp
                    </span>
                  </div>

                  <div class="min-h-0 flex-1 px-4 pb-4">
                    <LineChart data={sample.mqttPoints} x="at" y="amount" />
                  </div>
                </div>
              {:else}
                <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
                  {#each mqttFeed as msg, index (index)}
                    {@const open = mqttOpen === index}

                    <div
                      class="mx-1 mb-1 rounded-field {open
                        ? 'bg-base-200'
                        : 'hover:bg-base-200/60'}"
                    >
                      <button
                        type="button"
                        onclick={() => (mqttOpen = open ? null : index)}
                        class="flex w-full items-center gap-2 px-2 py-1
                          text-left"
                      >
                        <span
                          class="shrink-0 text-xs text-base-content/40"
                          style="font-variant-numeric: tabular-nums"
                        >
                          {msg.at}
                        </span>

                        <span
                          class="shrink-0 rounded-selector bg-base-300 px-2
                            text-xs text-base-content/55"
                        >
                          qos{msg.qos}
                        </span>

                        {#if msg.retained}
                          <Icon
                            icon="lucide:pin"
                            class="size-3 shrink-0 text-base-content/40"
                          />
                        {/if}

                        <span
                          class="min-w-0 flex-1 truncate font-mono text-xs
                            select-text"
                        >
                          {msg.payload}
                        </span>
                      </button>

                      {#if open}
                        <div class="px-2 pb-2">
                          <pre
                            class="max-h-64 overflow-auto rounded-field
                              bg-base-300 p-2 font-mono text-xs
                              select-text">{pretty(msg.payload)}</pre>

                          <div class="flex items-center gap-2 pt-1">
                            <span class="flex-1"></span>

                            <button
                              type="button"
                              onclick={() =>
                                navigator.clipboard.writeText(msg.payload)}
                              class="rounded-selector bg-base-300 px-2 py-1
                                text-xs hover:bg-base-100"
                            >
                              Copy
                            </button>
                          </div>
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}

              <footer
                class="shrink-0 border-t border-base-300/60 px-3 pt-2 pb-3"
              >
                <div class="flex items-center gap-2 pb-1">
                  <input
                    bind:value={mqttDraftTopic}
                    placeholder="Topic"
                    spellcheck="false"
                    class="min-w-0 flex-1 rounded-field bg-base-200 px-2 py-1
                      font-mono text-xs outline-none select-text
                      placeholder:text-base-content/30"
                  />

                  <select
                    bind:value={mqttQos}
                    aria-label="QoS"
                    class="cursor-pointer rounded-field bg-base-200 px-2 py-1
                      text-xs outline-none"
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
                      bind:checked={mqttRetain}
                      class="size-3 accent-primary"
                    />
                    retain
                  </label>

                  <button
                    type="button"
                    onclick={publish}
                    disabled={mqttDraftTopic.trim() === "" ||
                      mqttPayload.trim() === ""}
                    class="flex items-center gap-1 rounded-field bg-primary
                      px-3 py-1 text-xs text-primary-content
                      disabled:opacity-40"
                  >
                    <Icon icon="lucide:send" class="size-3" />
                    Publish
                  </button>
                </div>

                <textarea
                  bind:value={mqttPayload}
                  placeholder="Payload"
                  spellcheck="false"
                  rows="2"
                  onkeydown={event => {
                    if (event.key === "Enter" && event.ctrlKey) {
                      event.preventDefault()
                      publish()
                    }
                  }}
                  class="w-full resize-none rounded-field bg-base-200 px-2
                    py-1 font-mono text-xs outline-none select-text
                    placeholder:text-base-content/30"
                ></textarea>
              </footer>
            </section>
          </div>
          </DemoWindow>
        </div>
      </div>
    </section>

    <section class="pt-16 sm:pt-20" data-reveal use:reveal>
      <h2
        class="pt-3 font-display text-2xl font-bold tracking-tight sm:text-3xl"
      >
        Two databases, one window.
      </h2>

      <p class="max-w-2xl pt-4 text-base-content/65">
        Open staging beside production. Each one keeps its own tables, query
        buffer and history, so nothing you run in one shows up in the other.
        The session chip switches between them.
      </p>

      <p class="max-w-2xl pt-3 text-base-content/65">
        Point the schema tab at the other tab and it will tell you what drifted,
        then draft the migration. Anything that would drop a column or a table
        comes out commented, for you to decide.
      </p>
    </section>

    <section class="pt-16 sm:pt-20" data-reveal use:reveal>
      <h2
        class="pt-3 font-display text-2xl font-bold tracking-tight sm:text-3xl"
      >
        No menu diving.
      </h2>

      <div class="grid gap-4 pt-8 sm:grid-cols-2 lg:grid-cols-4">
        {#each machinery as card (card.title)}
          <article class="rounded-box bg-base-100 p-5 lift sm:p-6">
            <Icon icon={card.icon} class="size-5 text-accent" />

            <h3 class="pt-3 font-display text-lg font-medium">{card.title}</h3>

            <p class="pt-2 text-sm text-base-content/65">{card.text}</p>

            {#if card.keys}
              <Keycap keys={card.keys} class="pt-3" />
            {/if}
          </article>
        {/each}
      </div>
    </section>

    <section class="mt-16 grid gap-4 sm:mt-20 sm:grid-cols-3" data-reveal use:reveal>
      {#each [{ icon: "lucide:lock", title: "Read only is the default", text: "The server is asked to refuse writes until you flip one toggle, and it flips back on its own. Ask for manual commit and nothing lands until you say so." }, { icon: "lucide:shield", title: "Keys stay on the machine", text: "Windows seals saved logins with DPAPI. Elsewhere the file is plaintext, and the readme says so." }, { icon: "lucide:git-fork", title: "Diagrams without a server", text: "Draw an ERD offline. Publish it only when someone else needs the room." }] as card (card.title)}
        <article class="rounded-box bg-base-100 p-5 lift sm:p-6">
          <Icon icon={card.icon} class="size-5 text-accent" />

          <h3 class="pt-3 font-display text-lg font-medium">{card.title}</h3>

          <p class="pt-2 text-sm text-base-content/65">{card.text}</p>
        </article>
      {/each}
    </section>

    <section id="sync" class="grid gap-8 pt-16 sm:pt-20 lg:grid-cols-2">
      <div data-reveal use:reveal>
          <h2
        class="pt-3 font-display text-2xl font-bold tracking-tight sm:text-3xl"
      >
          An account is only for carrying settings across machines.
        </h2>

        <p class="pt-4 text-base-content/65">
          GPQL works forever without one. Sign in when you want the same setup
          on a second machine.
        </p>

        <ul class="space-y-2 pt-6">
          {#each ["Settings, theme and density", "Connections you have opened before", "Queries you kept", "ERD rooms, co-design with your team"] as item (item)}
            <li class="flex items-center gap-2 text-sm">
              <Icon icon="lucide:check" class="size-4 shrink-0 text-primary" />
              {item}
            </li>
          {/each}

          <li class="flex items-center gap-2 pt-2 text-sm text-base-content/55">
            <Icon icon="lucide:x" class="size-4 shrink-0 text-error" />
            Passwords. Those stay sealed on the machine that made them.
          </li>
        </ul>
      </div>

      <div class="rounded-box bg-base-100 p-5 lift sm:p-6" data-reveal use:reveal>
        <p class="text-sm text-base-content/45">Sync</p>

        <p class="pt-2 font-display text-3xl font-bold tracking-tight sm:text-4xl">
          Free, with an account
        </p>

        <p class="pt-3 text-sm text-base-content/65">
          Same settings, connections and saved queries on every machine. No
          plan, no card, no seat count.
        </p>

        <a
          href="/account"
          class="mt-5 block rounded-field bg-primary py-3 text-center text-sm
            text-primary-content transition-colors hover:bg-primary/90"
        >
          Sign in with GitHub
        </a>

        <p class="pt-3 text-xs text-base-content/45">
          Sign in once in the browser. The app picks it up on its own.
        </p>
      </div>
    </section>

    <footer
      class="mt-16 flex flex-wrap items-center gap-x-6 gap-y-3 border-t
        border-base-content/8 pt-8 text-xs text-base-content/40 sm:mt-20"
    >
      <span class="flex items-center gap-2 text-base-content/55">
        <Logo class="size-4" />
        GPQL
      </span>

      <a href="/download" class="hover:text-base-content">Download</a>
      <a href="/account" class="hover:text-base-content">Account</a>
      <a
        href="https://github.com/pleahmacaka/gpql"
        class="hover:text-base-content"
      >
        Source
      </a>

      <span class="w-full sm:ml-auto sm:w-auto">local first, your rows stay yours</span>
    </footer>
  </div>
</div>
