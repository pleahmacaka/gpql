<script lang="ts">
  import {
    DataGrid,
    Icon,
    Logo,
    ResultChart,
    SchemaBoard,
    SessionCard,
  } from "@gpql/ui"
  import { SvelteFlowProvider } from "@xyflow/svelte"

  import DemoWindow from "$lib/components/marketing/DemoWindow.svelte"
  import Sql from "$lib/components/marketing/Sql.svelte"
  import * as sample from "$lib/components/marketing/sample"
  import { reveal } from "$lib/reveal"

  let draft = $state({ ...sample.draft })
  let readOnly = $state(true)
  let dataView = $state<"table" | "chart">("table")

  const SUMMARY =
    "A SQL client that keeps everything on your disk. Fourteen databases, one window, read only until you say otherwise."
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
        class="flex items-center gap-1 rounded-box bg-base-100/80 p-1.5 pl-3
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

        {#each [{ href: "#engines", label: "Databases" }, { href: "#screens", label: "Screens" }, { href: "#sync", label: "Sync" }] as link (link.href)}
          <a
            href={link.href}
            class="hidden rounded-field px-3 py-1.5 text-base-content/60
              transition-colors hover:bg-base-200 hover:text-base-content
              sm:block"
          >
            {link.label}
          </a>
        {/each}

        <span class="flex-1"></span>

        <a
          href="/download"
          class="hidden rounded-field px-3 py-1.5 text-base-content/60
            transition-colors hover:bg-base-200 hover:text-base-content sm:block"
        >
          Download
        </a>

        <a
          href="/account"
          class="rounded-field bg-base-200 px-3 py-1.5 transition-colors
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
            class="flex items-center gap-2 rounded-field bg-primary px-4 py-2.5
              text-sm text-primary-content transition-colors hover:bg-primary/90"
          >
            <Icon icon="lucide:download" class="size-4" />
            Download for Windows
          </a>

          <a
            href="#engines"
            class="rounded-field bg-base-100 px-4 py-2.5 text-sm hairline
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
                labels={{ keys: "" }}
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
            class="flex items-center gap-3 rounded-field bg-base-100 px-3 py-2.5
              hairline transition-colors hover:bg-base-300"
          >
            <Icon icon={engine.icon} class="size-4 shrink-0" />

            <span class="min-w-0 flex-1">
              <span class="block truncate text-sm">{engine.name}</span>

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
                    class="flex items-center gap-2 rounded-field px-2 py-1.5
                      text-sm {table.name === 'message'
                      ? 'bg-primary/10 text-primary'
                      : ''}"
                  >
                    <Icon icon="lucide:table-2" class="size-3.5 opacity-60" />
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
                      p-0.5"
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
                        <Icon icon={option.icon} class="size-3.5" />
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
                  <Icon icon="lucide:sparkles" class="size-3.5 text-accent" />

                  <span class="flex-1 text-sm text-base-content/70">
                    {sample.ask[0].text}
                  </span>
                </div>

                <Sql code={sample.ask[1].text} />

                <p
                  class="flex items-center gap-1.5 font-mono text-xs
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

    <section class="pt-16 sm:pt-20" data-reveal use:reveal>
      <h2
        class="pt-3 font-display text-2xl font-bold tracking-tight sm:text-3xl"
      >
        Two databases, one window.
      </h2>

      <p class="max-w-2xl pt-4 text-base-content/65">
        Open staging beside production. Each tab keeps its own tables, query
        buffer and history, so nothing you run in one shows up in the other.
      </p>

      <p class="max-w-2xl pt-3 text-base-content/65">
        Point the schema tab at the other tab and it will tell you what drifted,
        then draft the migration. Anything that would drop a column or a table
        comes out commented, for you to decide.
      </p>
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
          class="mt-5 block rounded-field bg-primary py-2.5 text-center text-sm
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
