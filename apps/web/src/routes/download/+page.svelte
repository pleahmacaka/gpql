<script lang="ts">
  import Logo from "@gpql/ui/controls/Logo.svelte"
  import { Icon } from "@gpql/ui/icons/index.ts"

  const REPO = "https://github.com/pleahmacaka/gpql"
</script>

<svelte:head>
  <title>Download GPQL</title>
  <meta
    name="description"
    content="Download GPQL for Windows 10 and 11, or build it from source."
  />
</svelte:head>

<div class="min-h-screen bg-base-100">
  <main class="mx-auto max-w-xl px-4 py-12 sm:px-6 sm:py-16">
    <a href="/" class="flex w-fit items-center gap-2 font-semibold">
      <span aria-hidden="true"><Logo class="size-6" /></span>
      GPQL
    </a>

    <h1 class="pt-12 text-3xl font-bold tracking-tight sm:text-4xl">
      Download GPQL for Windows
    </h1>

    <p class="pt-4 text-pretty text-base-content/80">
      Every tagged release is built by GitHub Actions and published with a setup
      file and an MSI installer. Both are 64-bit.
    </p>

    <div class="flex flex-wrap gap-2 pt-8">
      <a href="{REPO}/releases/latest" class="btn btn-primary">
        <Icon icon="lucide:download" class="size-4" />
        Download the latest release
      </a>

      <a href="{REPO}/releases" class="btn font-normal btn-ghost">
        <Icon icon="simple-icons:github" class="size-4" />
        All releases and notes
      </a>
    </div>

    <h2 class="pt-12 text-lg font-semibold">What it needs</h2>

    <p class="pt-2 text-pretty text-base-content/80">
      Windows 10 or 11, 64-bit, with the WebView2 runtime. Windows 11 ships
      with it and current Windows 10 installs have it too.
    </p>

    <h2 class="pt-12 text-lg font-semibold">Build it yourself</h2>

    <p class="pt-2 text-pretty text-base-content/80">
      You need Bun and a Rust toolchain, plus the
      <a
        href="https://v2.tauri.app/start/prerequisites/"
        class="link link-primary"
      >
        Tauri prerequisites for Windows
      </a>.
    </p>

    <pre
      class={[
        "mt-4 overflow-x-auto rounded-box bg-base-200",
        "p-4 text-sm leading-relaxed",
      ]}><code
        >git clone {REPO}
cd gpql
bun install
bun run build</code
      ></pre>

    <p class="pt-4 text-sm text-pretty text-base-content/70">
      The installers land in
      <code class="break-all">apps/desktop/src-tauri/target/release/bundle</code>.
    </p>
  </main>
</div>
