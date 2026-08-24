<script lang="ts">
</script>

<svelte:head>
  <title>Download GPQL</title>
</svelte:head>

<div class="mx-auto max-w-lg px-6 py-16">
  <a href="/" class="font-display text-base font-medium">GPQL</a>

  <h1 class="pt-8 font-display text-3xl font-bold tracking-tight">
    Windows build
  </h1>

  <p class="pt-3 text-base-content/65">
    Signed installers land here on the first release. Until then, build it from source:
  </p>

  <pre
    class="mt-5 overflow-x-auto rounded-box bg-base-100 p-4 font-mono text-xs lift"><code
      >git clone https://github.com/gpql/gpql
cd gpql
bun install
bun run app</code
    ></pre>

  <p class="pt-4 text-xs text-base-content/45">
    Needs Rust and the WebView2 runtime, both already on most Windows machines.
  </p>
</div>
