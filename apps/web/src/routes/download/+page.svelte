<script lang="ts">
  import { Icon, Logo } from "@gpql/ui"

  const REPO = "https://github.com/pleahmacaka/gpql"
</script>

<svelte:head>
  <title>Download GPQL</title>
</svelte:head>

<div class="mx-auto max-w-lg px-4 py-12 sm:px-6 sm:py-16">
  <a href="/" class="flex items-center gap-2 font-display text-base font-medium">
    <Logo class="size-5" />
    GPQL
  </a>

  <h1 class="pt-8 font-display text-3xl font-bold tracking-tight">
    Windows build
  </h1>

  <p class="pt-3 text-base-content/65">
    Every tagged release is built by GitHub Actions and published with its
    installer attached.
  </p>

  <a
    href="{REPO}/releases/latest"
    class="mt-6 flex items-center justify-center gap-2 rounded-field bg-primary
      py-3 text-sm text-primary-content transition-colors hover:bg-primary/90"
  >
    <Icon icon="lucide:download" class="size-4" />
    Download the latest release
  </a>

  <a
    href="{REPO}/releases"
    class="mt-2 flex items-center justify-center gap-2 rounded-field bg-base-100
      py-3 text-sm hairline transition-colors hover:bg-base-300"
  >
    <Icon icon="simple-icons:github" class="size-4" />
    All releases and notes
  </a>

  <h2 class="pt-10 font-display text-lg font-medium">Or build it yourself</h2>

  <pre
    class="mt-3 overflow-x-auto rounded-box bg-base-100 p-4 font-mono text-xs lift"><code
      >git clone {REPO}
cd gpql
bun install
bun run build</code
    ></pre>

  <p class="pt-4 text-xs text-base-content/45">
    Needs Rust and the WebView2 runtime, both already on most Windows machines.
  </p>
</div>
