<script lang="ts">
  import Keycap from "@gpql/ui/controls/Keycap.svelte"
  import { Icon } from "@gpql/ui/icons/index.ts"
  import type { Snippet } from "svelte"

  const PLAN = [
    { node: "Limit", depth: "pl-0", cost: "w-4" },
    { node: "Sort", depth: "pl-4", cost: "w-8" },
    { node: "Seq Scan on pin", depth: "pl-8", cost: "w-16" },
  ]
</script>

{#snippet tile(title: string, line: string, art: Snippet)}
  <li class="hud hud-small flex flex-col gap-6 bg-base-100 p-5 hover:hud-lit">
    <div aria-hidden="true" class="flex h-20 items-center">
      {@render art()}
    </div>

    <div>
      <h3 class="text-base font-semibold">{title}</h3>
      <p class="pt-1 text-sm text-pretty text-base-content/70">{line}</p>
    </div>
  </li>
{/snippet}

{#snippet chip(label: string)}
  <span class="px-2 py-1 text-xs text-base-content/80 hairline">{label}</span>
{/snippet}

{#snippet palette()}
  <div class="w-full space-y-2">
    <Keycap keys={["ctrl", "k"]} />

    <p class="flex items-center gap-2 text-xs text-base-content/80">
      <Icon icon="lucide:table-2" class="size-4 text-primary" />
      message
    </p>
  </div>
{/snippet}

{#snippet plan()}
  <div class="w-full space-y-1 text-xs">
    {#each PLAN as step (step.node)}
      <p class={["flex items-center gap-2", step.depth]}>
        <span class="text-base-content/70">
          {step.depth === "pl-0" ? "" : "└"}
        </span>
        <span class="min-w-0 flex-1 truncate">{step.node}</span>
        <span class={["h-2 shrink-0 bg-primary", step.cost]}></span>
      </p>
    {/each}
  </div>
{/snippet}

{#snippet diff()}
  <div class="w-full space-y-1 text-xs">
    <p class="truncate text-success">+ create index on message (sent_at);</p>
    <p class="truncate text-base-content/70">-- drop table legacy_pin;</p>
  </div>
{/snippet}

{#snippet exports()}
  <div class="flex items-center gap-2">
    <Icon icon="lucide:file-down" class="size-6 text-primary" />
    {@render chip("CSV")}
    {@render chip("JSON")}
    {@render chip("INSERT")}
  </div>
{/snippet}

{#snippet models()}
  <div class="flex items-center gap-4">
    <Icon icon="simple-icons:openai" class="size-8" />
    <Icon icon="simple-icons:openrouter" class="size-8" />
    <Icon icon="lucide:sparkles" class="size-6 text-accent" />
  </div>
{/snippet}

{#snippet languages()}
  <div class="flex items-center gap-2">
    {@render chip("EN")}
    {@render chip("KO")}
    {@render chip("JA")}
    {@render chip("ZH")}
  </div>
{/snippet}

{#snippet local()}
  <div class="flex items-center gap-3">
    <Icon icon="lucide:hard-drive" class="size-8 text-primary" />
    <span class="w-10 border-t border-dashed border-base-content/30"></span>
    <Icon icon="lucide:cloud" class="size-8 text-base-content/70" />
  </div>
{/snippet}

{#snippet size()}
  <p class="text-5xl font-extrabold tracking-tight tabular-nums">
    17<span class="pl-1 text-2xl text-base-content/70">MB</span>
  </p>
{/snippet}

<section
  id="details"
  aria-labelledby="details-title"
  class="border-t border-base-content/10"
>
  <div class="mx-auto max-w-6xl px-4 py-24 sm:px-6">
    <p class="flex items-center gap-3 text-sm text-primary">
      <span class="size-2 bg-primary"></span>
      Details
    </p>

    <h2
      id="details-title"
      class="pt-4 text-4xl font-extrabold tracking-tight sm:text-6xl"
    >
      Also in the box.
    </h2>

    <ul class="grid gap-2 pt-12 sm:grid-cols-2 lg:grid-cols-4">
      {@render tile(
        "Ctrl+K palette",
        "Tables, saved queries, toggles and settings.",
        palette,
      )}
      {@render tile(
        "EXPLAIN as a tree",
        "ANALYZE refuses anything that writes.",
        plan,
      )}
      {@render tile(
        "Diff to migration",
        "Two open connections, drops commented out.",
        diff,
      )}
      {@render tile(
        "Export",
        "A table or a result, filters included.",
        exports,
      )}
      {@render tile(
        "Your own AI key",
        "Any OpenAI-compatible provider, or OpenRouter.",
        models,
      )}
      {@render tile(
        "Four languages",
        "English, Korean, Japanese and Chinese.",
        languages,
      )}
      {@render tile(
        "Local first",
        "Sync is free with GitHub. Passwords stay on this PC.",
        local,
      )}
      {@render tile(
        "Setup file",
        "About 17 MB. Tauri 2, drawn with WebView2.",
        size,
      )}
    </ul>
  </div>
</section>
