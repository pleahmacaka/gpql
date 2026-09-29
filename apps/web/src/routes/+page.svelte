<script lang="ts">
  import Logo from "@gpql/ui/controls/Logo.svelte"
  import { Icon } from "@gpql/ui/icons/index.ts"
  import { scramble } from "@gpql/ui/motion/scramble.ts"

  import AsciiMark from "$lib/components/marketing/AsciiMark.svelte"
  import Backends from "$lib/components/marketing/Backends.svelte"
  import Hero from "$lib/components/marketing/Hero.svelte"
  import Hops from "$lib/components/marketing/Hops.svelte"
  import Specs from "$lib/components/marketing/Specs.svelte"
  import Stage from "$lib/components/marketing/Stage.svelte"

  const REPO = "https://github.com/pleahmacaka/gpql"
  const RELEASE = `${REPO}/releases/latest`

  const SUMMARY =
    "GPQL is a desktop client for PostgreSQL, MySQL, SQLite, DuckDB, " +
    "ClickHouse, InfluxDB, Neo4j and more, plus MQTT brokers and S3 " +
    "buckets. Connections, history and saved queries live on your machine " +
    "unless you turn on sync."

  const SECTIONS = [
    { href: "#tour", label: "Tour" },
    { href: "#backends", label: "Backends" },
    { href: "#reach", label: "Reach" },
    { href: "#details", label: "Details" },
  ]

  const FACTS = ["Windows 10 and 11", "64-bit", "Setup or MSI", "Tauri 2"]

  let rushing = false

  const rush = (on: boolean) => () => {
    rushing = on
  }
</script>

<svelte:head>
  <title>GPQL: desktop client for databases, brokers and buckets</title>
  <meta name="description" content={SUMMARY} />
</svelte:head>

<div class="relative isolate min-h-screen">
  <a
    href="#main"
    class={[
      "sr-only focus:not-sr-only focus:fixed focus:top-4 focus:left-4",
      "focus:z-50 focus:bg-base-100 focus:px-4 focus:py-2",
    ]}
  >
    Skip to content
  </a>

  <header class="pane sticky top-0 z-40">
    <div class="navbar mx-auto min-h-16 max-w-7xl gap-2 px-4 sm:px-6">
      <a href="/" class="flex items-center gap-2 text-base font-semibold">
        <span aria-hidden="true"><Logo class="size-6" /></span>
        GPQL
      </a>

      <nav aria-label="Sections" class="hidden gap-1 pl-6 md:flex">
        {#each SECTIONS as section (section.href)}
          <a href={section.href} class="btn font-normal btn-ghost btn-sm">
            {section.label}
          </a>
        {/each}
      </nav>

      <span class="flex-1"></span>

      <a href="/account" class="btn font-normal btn-ghost btn-sm">Account</a>

      <a href={RELEASE} class="btn btn-sm btn-primary">
        <Icon icon="lucide:download" class="size-4" />
        Download
      </a>
    </div>
  </header>

  <main id="main" tabindex="-1" class="outline-none">
    <Hero release={RELEASE} repo={REPO} />

    <Stage />

    <Backends />

    <Hops />

    <Specs />

    <section
      id="download"
      aria-labelledby="download-title"
      class="border-t border-base-content/10"
    >
      <div
        class={[
          "mx-auto grid max-w-6xl items-center gap-12 px-4 py-24 sm:px-6",
          "lg:grid-cols-12",
        ]}
      >
        <div
          class={[
            "relative isolate flex justify-center py-8",
            "lg:order-last lg:col-span-5",
          ]}
        >
          <div
            aria-hidden="true"
            class="absolute inset-0 -z-10 mask-radial-from-40%"
          >
            <div class="grid-field absolute inset-0"></div>
          </div>

          <AsciiMark busy={() => rushing} />
        </div>

        <div class="lg:col-span-7">
          <p class="flex items-center gap-3 text-sm text-primary">
            <span class="size-2 bg-primary"></span>
            Download
          </p>

          <h2
            id="download-title"
            use:scramble={{ onView: true }}
            class={[
              "pt-4 text-5xl font-extrabold tracking-tight text-balance",
              "sm:text-7xl",
            ]}
          >
            Get GPQL.
          </h2>

          <ul class="flex flex-wrap gap-2 pt-8 text-sm text-base-content/80">
            {#each FACTS as fact (fact)}
              <li class="px-3 py-1 tabular-nums hairline">{fact}</li>
            {/each}
          </ul>

          <div class="flex flex-wrap gap-2 pt-10">
            <a
              href={RELEASE}
              class="btn btn-lg btn-primary"
              onpointerenter={rush(true)}
              onpointerleave={rush(false)}
              onfocus={rush(true)}
              onblur={rush(false)}
            >
              <Icon icon="lucide:download" class="size-5" />
              Download the latest release
            </a>

            <a href="/download" class="btn font-normal btn-ghost btn-lg">
              Build from source
            </a>
          </div>
        </div>
      </div>
    </section>
  </main>

  <footer class="border-t border-base-content/10">
    <div
      class={[
        "mx-auto flex max-w-6xl flex-wrap items-center gap-x-6 gap-y-2",
        "px-4 py-8 text-sm text-base-content/70 sm:px-6",
      ]}
    >
      <span class="flex items-center gap-2 font-semibold text-base-content">
        <span aria-hidden="true"><Logo class="size-4" plain /></span>
        GPQL
      </span>

      <a href="/download" class="link link-hover">Download</a>
      <a href="{REPO}/releases" class="link link-hover">Release notes</a>
      <a href={REPO} class="link link-hover">Source</a>
      <a href="/account" class="link link-hover">Account</a>
    </div>
  </footer>
</div>
