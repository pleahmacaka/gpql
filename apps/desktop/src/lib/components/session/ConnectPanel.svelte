<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Segmented as Segmented } from "@gpql/ui"
  import type { Mode, SessionConfig } from "$lib/types"
  import { workspace } from "$lib/session/workspace.svelte"

  import NewSession from "./NewSession.svelte"
  import QuickConnect from "./QuickConnect.svelte"
  import RecentList from "./RecentList.svelte"

  let seed = $state<SessionConfig | null>(null)

  const titles: Record<Mode, string> = {
    new: m.panel_new(),
    quick: m.panel_quick(),
    recent: m.panel_recent(),
  }

  function handoff(config: SessionConfig) {
    seed = config
    workspace.mode = "new"
  }
</script>

<section class="rounded-box bg-base-100 p-4 lift">
  <h1 class="mb-3 text-sm font-medium">{titles[workspace.mode]}</h1>

  <div class="pb-2">
    <Segmented
      bind:value={() => workspace.mode, next => (workspace.mode = next as Mode)}
      options={[
        { value: "new", label: m.mode_new() },
        { value: "quick", label: m.mode_quick() },
        { value: "recent", label: m.mode_recent() },
      ]}
    />
  </div>

  {#if workspace.mode === "new"}
    {#key seed}
      <NewSession {seed} />
    {/key}
  {:else if workspace.mode === "quick"}
    <QuickConnect onhandoff={handoff} />
  {:else}
    <RecentList />
  {/if}
</section>
