<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import ResultGrid from "$lib/components/data/ResultGrid.svelte"
  import { Icon } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"

  import AskBar from "./AskBar.svelte"
  import SavedQueries from "./SavedQueries.svelte"
  import SqlEditor from "./SqlEditor.svelte"

  let picked = $derived(workspace.selection.end > workspace.selection.start)

  let status = $derived(
    workspace.queryError
      ? workspace.queryError
      : workspace.busy
        ? m.running()
        : workspace.queryRan
          ? m.rows_count({ count: workspace.queryResult?.rows.length ?? 0 })
          : m.nothing_run(),
  )
</script>

<div class="flex h-full gap-2 p-2">
  <SavedQueries />

  <section class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-baseline gap-4 px-4 pt-2 pb-1">
      <h2 class="text-sm font-medium">Query</h2>

      <span class="text-xs text-base-content/40">
        {picked ? m.runs_selection() : m.runs_everything()}
      </span>

      <span class="text-xs text-base-content/40">{m.clears()}</span>
    </header>

    <AskBar />

    <div class="mx-3 rounded-field bg-base-200 px-3 pt-2 pb-1.5">
      <SqlEditor
        bind:value={workspace.sql}
        bind:selection={workspace.selection}
        onrun={() => workspace.run()}
        onclear={() => workspace.clearQuery()}
      />

      <p
        class="flex items-center gap-1.5 pt-1 text-xs {workspace.queryError
          ? 'text-error'
          : 'text-base-content/40'}"
      >
        <Icon icon="lucide:play" class="size-3" />
        {status}
      </p>
    </div>

    <ResultGrid result={workspace.queryResult} empty="" />
  </section>
</div>
