<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon, Segmented } from "@gpql/ui"
  import type { Mode, SessionConfig } from "$lib/types"
  import { workspace } from "$lib/session/workspace.svelte"

  import ErdStart from "$lib/components/erd/ErdStart.svelte"

  import NewSession from "./NewSession.svelte"
  import QuickConnect from "./QuickConnect.svelte"
  import ConnectionList from "./ConnectionList.svelte"

  let seed = $state<SessionConfig | null>(null)
  let from = $state<Mode>("recent")

  $effect(() => {
    if (workspace.mode !== "new") {
      from = workspace.mode
    }
  })

  const titles: Record<Mode, string> = {
    new: m.panel_new(),
    quick: m.panel_quick(),
    recent: m.panel_recent(),
    erd: m.panel_erd(),
  }

  function handoff(config: SessionConfig) {
    seed = config
    workspace.mode = "new"
  }

  function back() {
    seed = null
    workspace.editing = null
    workspace.mode = from
  }
</script>

<section class="rounded-box bg-base-100 p-4 lift">
  <div class="mb-3 flex items-center gap-2">
    {#if workspace.mode === "new"}
      <button
        type="button"
        aria-label={m.back()}
        onclick={back}
        class="-ml-1 rounded-selector p-1 text-base-content/40
          hover:text-base-content"
      >
        <Icon icon="lucide:arrow-left" class="size-4" />
      </button>
    {/if}

    <h1 class="flex-1 text-sm font-medium">
      {workspace.mode === "new" && workspace.editing
        ? m.menu_edit()
        : titles[workspace.mode]}
    </h1>

    {#if workspace.session}
      <button
        type="button"
        aria-label={m.close()}
        onclick={() => (workspace.connecting = false)}
        class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
      >
        <Icon icon="lucide:x" class="size-4" />
      </button>
    {/if}
  </div>

  <div class="pb-2">
    <Segmented
      onpick={next => {
        workspace.editing = null
        seed = null
        workspace.mode = next as Mode
      }}
      value={workspace.mode === "new" ? "recent" : workspace.mode}
      options={[
        { value: "recent", label: m.mode_recent() },
        { value: "quick", label: m.mode_quick() },
        { value: "erd", label: m.mode_erd() },
      ]}
    />
  </div>

  {#if workspace.mode === "new"}
    {#key seed}
      <NewSession {seed} onsaved={back} />
    {/key}
  {:else if workspace.mode === "quick"}
    <QuickConnect onhandoff={handoff} />
  {:else if workspace.mode === "recent"}
    <ConnectionList onedit={handoff} />
  {:else}
    <ErdStart />
  {/if}
</section>
