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
  import * as sample from "$lib/components/marketing/sample"
  import { reveal } from "$lib/reveal"

  let draft = $state({ ...sample.draft })
  let readOnly = $state(true)
  let dataView = $state<"table" | "chart">("table")

  const SUMMARY =
    "GPQL is a SQL client that keeps everything on your disk. Fourteen databases, each through its own driver. Your logins are sealed by the OS, your queries never leave the machine, and read only is on until you turn it off."
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

  <div class="relative mx-auto max-w-5xl px-6 pb-24">
    <div class="sticky top-0 z-40 -mx-6 px-6 pt-2 pb-3">
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
            class="rounded-field px-3 py-1.5 text-base-content/60
              transition-colors hover:bg-base-200 hover:text-base-content"
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

    <header class="grid items-center gap-10 pt-8 pb-14 lg:grid-cols-2">
      <div data-reveal use:reveal>
        <p class="font-mono text-xs text-base-content/45">
          fourteen databases, one window, none of them ours
        </p>

        <h1
          class="pt-4 font-display text-5xl leading-tight font-bold tracking-tight"
        >
          Open the database.<br />
          Read the rows.<br />
          <span class="text-base-content/40">Close it.</span>
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

        <div class="flex gap-4 pt-5 font-mono text-xs text-base-content/40">
          <span>tab moves</span>
          <span>GPQL checks as you type</span>
          <span>return connects</span>
        </div>
      </div>

      <div class="relative" data-reveal use:reveal>
        <div
          aria-hidden="true"
          class="absolute -inset-6 rounded-box bg-primary/10 blur-3xl"
        ></div>

        <DemoWindow chip="no session" chipIcon="lucide:plus">
          <div class="bg-base-200 px-8 py-6">
            <section class="mx-auto w-96 rounded-box bg-base-100 p-4 lift">
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

    <div
      class="flex flex-wrap gap-x-8 gap-y-2 pb-4 font-mono text-xs
        text-base-content/45"
      data-reveal
      use:reveal
    >
      <span>14 drivers, each the vendor's own</span>
      <span>3 screens</span>
      <span>0 bytes of telemetry</span>
      <span>read only until you say otherwise</span>
    </div>

    <section id="engines" class="pt-10" data-reveal use:reveal>
      <p class="font-mono text-xs text-base-content/45">drivers</p>

      <h2 class="pt-3 font-display text-3xl font-bold tracking-tight">
        Every database speaks for itself.
      </h2>

      <p class="max-w-2xl pt-4 text-base-content/65">
        No wrapper protocol, no hand-rolled HTTP. Each backend goes through the
        driver its own people maintain, so the wire behaviour is theirs and the
        bugs are the ones you can look up.
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

      <p class="pt-4 font-mono text-xs text-base-content/40">
        cloudflare d1 has no rust driver, so that one is the documented
        exception: plain rest, said out loud in the readme
      </p>
    </section>

    <div class="mt-16 h-px rule"></div>

    <section id="screens" class="pt-16">
      <div data-reveal use:reveal>
        <p class="font-mono text-xs text-base-content/45">read</p>

        <h2 class="pt-3 font-display text-3xl font-bold tracking-tight">
          Three screens. No fourth one.
        </h2>
      </div>

      <div class="space-y-12 pt-8">
        <article data-reveal use:reveal>
          <div class="flex flex-wrap items-baseline gap-3 pb-3">
            <h3 class="font-display text-lg font-medium">Data</h3>

            <p class="text-sm text-base-content/55">
              Rows and columns both virtualized, edits staged until you apply
              them, and a minimap when the result runs off both edges.
            </p>
          </div>

          <div
            class="rounded-box bg-linear-to-b from-base-100 to-base-200 p-3
              ring-1 ring-base-content/5"
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
              Highlighting per dialect from tree-sitter, completion from a real
              language server, and an ask bar that turns a sentence into SQL
              through your own key.
            </p>
          </div>

          <div
            class="rounded-box bg-linear-to-b from-base-100 to-base-200 p-3
              ring-1 ring-base-content/5"
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

                  <span class="font-mono text-xs text-base-content/40">
                    your key, your endpoint
                  </span>
                </div>

                <pre
                  class="flex-1 overflow-auto rounded-field bg-base-200 p-4
                    font-mono text-xs leading-relaxed text-base-content/80">{sample
                    .ask[1].text}</pre>

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
              keys, and groupable by hand or by asking a model to read the
              shape.
            </p>
          </div>

          <div
            class="rounded-box bg-linear-to-b from-base-100 to-base-200 p-3
              ring-1 ring-base-content/5"
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

    <section class="mt-20 grid gap-4 sm:grid-cols-3" data-reveal use:reveal>
      {#each [{ icon: "lucide:lock", title: "Read only is the default", text: "GPQL asks the server to refuse every write before it runs a statement. Turning it off is one toggle, and the toggle says so out loud." }, { icon: "lucide:shield", title: "Keys stay on the machine", text: "Windows seals saved logins with DPAPI. Elsewhere the file is plaintext in your home folder, and the readme says exactly that." }, { icon: "lucide:git-fork", title: "Diagrams without a server", text: "Draw an ERD offline. Publish it only when you want someone else in the room." }] as card (card.title)}
        <article class="rounded-box bg-base-100 p-6 lift">
          <Icon icon={card.icon} class="size-5 text-accent" />

          <h3 class="pt-3 font-display text-lg font-medium">{card.title}</h3>

          <p class="pt-2 text-sm text-base-content/65">{card.text}</p>
        </article>
      {/each}
    </section>

    <section id="sync" class="grid gap-8 pt-20 lg:grid-cols-2">
      <div data-reveal use:reveal>
        <p class="font-mono text-xs text-base-content/45">sync</p>

        <h2 class="pt-3 font-display text-3xl font-bold tracking-tight">
          An account is only for carrying settings across machines.
        </h2>

        <p class="pt-4 text-base-content/65">
          GPQL works forever without one. Sign in when you want the same setup on
          a second machine, sign out and the app keeps working exactly as it did.
        </p>

        <ul class="space-y-2 pt-6">
          {#each ["Settings: theme, density, read only", "Connections you have opened before", "Queries you kept", "ERD rooms, so a diagram can have two people in it"] as item (item)}
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

      <div class="rounded-box bg-base-100 p-6 lift" data-reveal use:reveal>
        <p class="text-sm text-base-content/45">Sync</p>

        <p class="pt-2 font-display text-4xl font-bold tracking-tight">
          Free, with an account
        </p>

        <p class="pt-3 text-sm text-base-content/65">
          Sign in and every machine you sign in on lands on the same settings,
          connections and saved queries. No plan, no card, no seat count.
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
      class="mt-20 flex flex-wrap items-center gap-6 border-t
        border-base-content/8 pt-8 text-xs text-base-content/40"
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

      <span class="ml-auto">local first, your rows stay yours</span>
    </footer>
  </div>
</div>
