<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Segmented as Segmented } from "@gpql/ui"
  import type { Mode, SessionConfig } from "$lib/types"
  import { workspace } from "$lib/session/workspace.svelte"

  import ErdStart from "$lib/components/erd/ErdStart.svelte"

  import NewSession from "./NewSession.svelte"
  import QuickConnect from "./QuickConnect.svelte"
  import RecentList from "./RecentList.svelte"

  let seed = $state<SessionConfig | null>(null)

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
</script>

<section class="rounded-box bg-base-100 p-4 lift">
  <h1 class="mb-3 text-sm font-medium">{titles[workspace.mode]}</h1>

  <div class="pb-2">
    <Segmented
      onpick={next => (workspace.mode = next as Mode)}
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
      <NewSession {seed} />
    {/key}
  {:else if workspace.mode === "quick"}
    <QuickConnect onhandoff={handoff} />
  {:else if workspace.mode === "recent"}
    <RecentList />
  {:else}
    <ErdStart />
  {/if}
</section>
