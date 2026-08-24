<script lang="ts">
  import {
    DataGrid,
    Icon,
    ResultChart,
    SchemaBoard,
    SessionCard,
  } from "@gpql/ui"
  import { SvelteFlowProvider } from "@xyflow/svelte"

  import DemoWindow from "$lib/components/marketing/DemoWindow.svelte"
  import * as sample from "$lib/components/marketing/sample"

  let draft = $state({ ...sample.draft })
  let readOnly = $state(true)
  let dataView = $state<"table" | "chart">("table")
</script>

<svelte:head>
  <title>GPQL</title>
  <meta name="description" content="GPQL is a SQL client that keeps everything on your disk. Your logins are sealed by the OS, your queries never leave the machine, and read only is on until you turn it off." />
</svelte:head>

<div class="mx-auto max-w-5xl px-6 pb-24">
  <nav class="flex items-center gap-6 py-6 text-sm">
    <span class="font-display text-base font-medium">GPQL</span>

    <a href="#screens" class="text-base-content/55 hover:text-base-content">
      Screens
    </a>

    <a href="#sync" class="text-base-content/55 hover:text-base-content">
      Sync
    </a>

    <a
      href="/account"
      class="ml-auto rounded-field bg-base-100 px-3 py-1.5 hairline
        hover:bg-base-300"
    >
      Account
    </a>
  </nav>

  <header class="grid items-center gap-10 py-10 lg:grid-cols-2">
    <div>
      <p class="font-mono text-xs text-base-content/45">postgres, mysql and ten more, on this machine</p>

      <h1
        class="pt-4 font-display text-5xl leading-tight font-bold tracking-tight"
      >
        Open the database.<br />
        Read the rows.<br />
        <span class="text-base-content/40">Close it.</span>
      </h1>

      <p class="max-w-md pt-5 text-base-content/65">GPQL is a SQL client that keeps everything on your disk. Your logins are sealed by the OS, your queries never leave the machine, and read only is on until you turn it off.</p>

      <div class="flex flex-wrap items-center gap-3 pt-7">
        <a
          href="/download"
          class="rounded-field bg-primary px-4 py-2.5 text-sm text-primary-content
            hover:brightness-105"
        >
          Download for Windows
        </a>

        <a
          href="#sync"
          class="rounded-field bg-base-100 px-4 py-2.5 text-sm hairline
            hover:bg-base-300"
        >
          How sync works
        </a>
      </div>

      <div class="flex gap-4 pt-5 font-mono text-xs text-base-content/40">
        <span>tab moves</span>
        <span>GPQL checks as you type</span>
        <span>return connects</span>
      </div>
    </div>

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
  </header>

  <section id="screens" class="pt-16">
    <p class="font-mono text-xs text-base-content/45">read</p>

    <h2 class="pt-3 font-display text-3xl font-bold tracking-tight">
      Three screens. No fourth one.
    </h2>

    <div class="space-y-10 pt-8">
      <article>
        <div class="flex flex-wrap items-baseline gap-3 pb-3">
          <h3 class="font-display text-lg font-medium">
            Data
          </h3>

          <p class="text-sm text-base-content/55">Every table, its row count, and the rows themselves. Nulls stay visibly null.</p>
        </div>

        <DemoWindow chip="smartfarmics" tab="Data">
          <div class="flex h-96 gap-2 bg-base-200 p-2">
            <aside class="w-52 shrink-0 rounded-box bg-base-100 p-2 lift">
              {#each sample.tables as table (table.name)}
                <div
                  class="flex items-center gap-2 rounded-field px-2 py-1.5 text-sm
                    {table.name === 'leaf_measurement'
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
                <h4 class="text-sm font-medium">leaf_measurement</h4>

                <span class="text-xs text-base-content/45">
                  {sample.columns.length} columns
                </span>

                <span class="flex-1"></span>

                <div
                  class="flex gap-1 self-center rounded-selector bg-base-200 p-0.5"
                >
                  {#each [{ id: "table", icon: "lucide:table-2" }, { id: "chart", icon: "lucide:bar-chart-3" }] as option (option.id)}
                    <button
                      type="button"
                      aria-label={option.id}
                      onclick={() => (dataView = option.id as "table" | "chart")}
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
      </article>

      <article>
        <div class="flex flex-wrap items-baseline gap-3 pb-3">
          <h3 class="font-display text-lg font-medium">
            Schema
          </h3>

          <p class="text-sm text-base-content/55">Tables laid out by what they point at, walkable with the arrow keys.</p>
        </div>

        <DemoWindow chip="smartfarmics" tab="Schema">
          <div class="h-96 bg-base-200 p-2">
            <div class="h-full overflow-hidden rounded-box bg-base-100 lift">
              <SvelteFlowProvider>
                <SchemaBoard tables={sample.tables} keyboard={false} />
              </SvelteFlowProvider>
            </div>
          </div>
        </DemoWindow>
      </article>
    </div>
  </section>

  <section class="mt-20 rounded-box bg-base-100 p-8 lift">
    <div class="flex items-start gap-4">
      <Icon icon="lucide:lock" class="mt-1 size-5 text-accent" />

      <div>
        <h2 class="font-display text-2xl font-bold tracking-tight">
          Read only is the default
        </h2>

        <p class="max-w-2xl pt-2 text-base-content/65">GPQL asks the server to refuse every write before it runs a single statement. Turning it off is one toggle, and the toggle says so out loud. A client that can drop a table by accident is not a client you want open next to production.</p>
      </div>
    </div>
  </section>

  <section id="sync" class="grid gap-8 pt-20 lg:grid-cols-2">
    <div>
      <p class="font-mono text-xs text-base-content/45">sync</p>

      <h2 class="pt-3 font-display text-3xl font-bold tracking-tight">
        An account is only for carrying settings across machines.
      </h2>

      <p class="pt-4 text-base-content/65">GPQL works forever without one. Sign in when you want the same setup on a second machine, sign out and the app keeps working exactly as it did.</p>

      <ul class="space-y-2 pt-6">
        {#each ["Settings: dark, compact, read only", "Connections you have opened before", "Queries you kept"] as item (item)}
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

    <div class="rounded-box bg-base-100 p-6 lift">
      <p class="text-sm text-base-content/45">Sync</p>

      <p class="pt-2 font-display text-4xl font-bold tracking-tight">
        Free, with an account
      </p>

      <p class="pt-3 text-sm text-base-content/65">Sign in and every machine you sign in on lands on the same settings, connections and saved queries. No plan, no card, no seat count.</p>

      <a
        href="/account"
        class="mt-5 block rounded-field bg-primary py-2.5 text-center text-sm
          text-primary-content hover:brightness-105"
      >
        Sign in with GitHub or Google
      </a>

      <p class="pt-3 text-xs text-base-content/45">Sign in once in the browser. The app picks it up on its own.</p>
    </div>
  </section>

  <footer class="flex gap-6 pt-20 text-xs text-base-content/40">
    <span>GPQL</span>
    <span>local first</span>
    <span>your rows stay yours</span>
  </footer>
</div>
