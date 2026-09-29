<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { tick } from "svelte"

  import { save } from "@tauri-apps/plugin-dialog"

  import {
    arrive,
    ConfirmDialog,
    depart,
    Icon,
    Lazy,
    type MenuItem,
    Panel,
    Segmented,
    scramble,
    tooltip,
  } from "@gpql/ui"

  import { workspace } from "$lib/session/workspace.svelte"

  import { chartLabels } from "$lib/components/query/chart"
  import FindBar from "$lib/components/shell/FindBar.svelte"
  import TabLayout from "$lib/components/shell/TabLayout.svelte"

  import DdlView from "./DdlView.svelte"
  import MqttView from "./MqttView.svelte"
  import ResultGrid from "./ResultGrid.svelte"
  import TableList from "./TableList.svelte"
  import TransactionBar from "./TransactionBar.svelte"

  type View = "table" | "chart"

  const VIEWS: View[] = ["table", "chart"]

  const TITLE = "max-w-fit min-w-20 flex-1 truncate text-sm font-semibold"

  let view = $state<View>("table")
  let mqtt = $derived(workspace.session?.kind === "mqtt")
  let s3 = $derived(workspace.session?.kind === "s3")

  let views = $derived([
    {
      value: "table",
      label: m.view_table(),
      icon: mqtt ? "lucide:rss" : "lucide:table-2",
    },
    { value: "chart", label: m.view_chart(), icon: "lucide:chart-column" },
  ])

  let table = $derived(workspace.browse.table)

  let icon = $derived(
    mqtt ? "lucide:radio" : s3 ? "lucide:package" : "lucide:table-2",
  )

  let asking = $state<{ retry?: () => void } | null>(null)
  let removing = $state<{ bucket: string; key: string } | null>(null)
  let term = $state("")
  let hit = $state(0)

  let hits = $derived.by(() => {
    const needle = term.trim().toLowerCase()
    const result = workspace.browse.result

    if (needle === "" || !result) {
      return []
    }

    const out: { row: number; column: number }[] = []

    result.rows.forEach((row, y) => {
      row.forEach((cell, x) => {
        if ((cell ?? "").toLowerCase().includes(needle)) {
          out.push({ row: y, column: x })
        }
      })
    })

    return out
  })

  let bump = $state(0)

  let spot = $derived.by(() => {
    void bump

    if (!workspace.finding) {
      return null
    }

    const found = hits[Math.min(hit, hits.length - 1)]

    return found ? { ...found } : null
  })

  $effect(() => {
    void term
    hit = 0
  })

  let loaded = $derived(workspace.browse.result?.rows.length ?? 0)

  let total = $derived(
    workspace.tables.find(entry => entry.name === table)?.rows ?? 0,
  )

  let filtered = $derived(
    Object.keys(workspace.browse.filters).length > 0 &&
      workspace.browse.serverSide,
  )

  // the table total says nothing about a filtered set, so do not pair them
  let counter = $derived(
    workspace.browse.error && loaded > 0
      ? m.rows_partial({ loaded })
      : filtered
        ? workspace.nouns.rowsFiltered(loaded)
        : workspace.browse.end
          ? workspace.nouns.rowsAll(loaded)
          : workspace.nouns.rowsLoaded(loaded, total),
  )

  function pick(next: string) {
    view = VIEWS.find(entry => entry === next) ?? view
  }

  function step(by: number) {
    if (hits.length > 0) {
      hit = (hit + by + hits.length) % hits.length
      bump++
    }
  }

  function ask(retry?: () => void) {
    asking = { retry }
  }

  function toggleWrites() {
    if (workspace.readOnly) {
      ask()

      return
    }

    void workspace.setReadOnly(true)
  }

  async function allowWrites() {
    const retry = asking?.retry

    asking = null

    try {
      await workspace.setReadOnly(false)
    } catch (failure) {
      workspace.error = String(failure)

      return
    }

    await tick()
    retry?.()
  }

  function objectActions(row: number): MenuItem[] {
    const bucket = workspace.browse.table
    const key = workspace.browse.result?.rows[row]?.[0]

    if (!bucket || !key) {
      return []
    }

    return [
      {
        label: m.s3_download(),
        icon: "lucide:download",
        run: () => downloadObject(bucket, key),
      },
      {
        label: m.s3_presign(),
        icon: "lucide:link",
        run: () => linkObject(bucket, key),
      },
      {
        label: m.s3_delete(),
        icon: "lucide:trash-2",
        danger: true,
        run: () => {
          if (workspace.readOnly) {
            ask(() => (removing = { bucket, key }))

            return
          }

          removing = { bucket, key }
        },
      },
    ]
  }

  async function downloadObject(bucket: string, key: string) {
    const connection = workspace.active
    const name = key.split("/").pop() ?? key
    const path = await save({ defaultPath: name })

    if (!path || !connection) {
      return
    }

    try {
      await connection.s3Download(bucket, key, path)
      workspace.notice = m.s3_saved({ path })
    } catch (failure) {
      workspace.error = String(failure)
    }
  }

  async function linkObject(bucket: string, key: string) {
    const connection = workspace.active

    if (!connection) {
      return
    }

    try {
      navigator.clipboard.writeText(await connection.s3Presign(bucket, key))
    } catch (failure) {
      workspace.error = String(failure)
    }
  }

  async function removeObject() {
    const target = removing

    removing = null

    if (!target || !workspace.active) {
      return
    }

    try {
      await workspace.active.s3Delete(target.bucket, target.key)
    } catch (failure) {
      workspace.error = String(failure)
    }
  }
</script>

{#snippet status()}
  {#if table && workspace.browse.result}
    <span class="shrink-0 truncate">{counter}</span>
  {/if}

  <TransactionBar />
{/snippet}

<TabLayout>
  {#snippet aside()}
    <TableList onblocked={() => ask()} />
  {/snippet}

  <Panel
    glass={false}
    label={table ?? workspace.nouns.panel()}
    class="min-w-0 flex-1"
    inner="overflow-hidden"
  >
    {#if workspace.ddl}
      <div
        in:arrive={{ from: "right" }}
        out:depart={{ to: "right" }}
        class="absolute inset-0 flex flex-col bg-base-100"
      >
        <DdlView />
      </div>
    {:else}
      <div
        in:arrive={{ from: "left" }}
        out:depart={{ to: "left" }}
        class="absolute inset-0 flex flex-col bg-base-100"
      >
        <header
          class={[
            "flex h-12 shrink-0 items-center gap-3 border-b",
            "border-base-content/10 pr-2 pl-4",
          ]}
        >
          <Icon {icon} class="size-4 shrink-0 text-base-content/60" />

          {#if table}
            {#key table}
              <h2 use:scramble={{ duration: 260 }} class={TITLE}>{table}</h2>
            {/key}
          {:else}
            <h2 class={TITLE}>{workspace.nouns.none()}</h2>
          {/if}

          {#if table && !workspace.finding}
            <span
              class={[
                "min-w-0 shrink truncate text-xs whitespace-nowrap",
                "text-base-content/70",
              ]}
            >
              {#if mqtt}
                {workspace.nouns.row(total)}
              {:else}
                {workspace.nouns.columns(
                  workspace.browse.result?.columns.length ?? 0,
                )}
              {/if}
            </span>
          {/if}

          <span class="flex-1"></span>

          {#if workspace.finding}
            <FindBar
              placeholder={m.find_rows()}
              bind:term
              index={hit}
              total={hits.length}
              onnext={() => step(1)}
              onprev={() => step(-1)}
              onclose={() => (workspace.finding = false)}
            />
          {/if}

          {#if table && !workspace.finding}
            <Segmented
              small
              label={m.view_label()}
              options={views}
              value={view}
              onpick={pick}
            />
          {/if}

          <button
            type="button"
            onclick={toggleWrites}
            aria-pressed={!workspace.readOnly}
            use:tooltip={workspace.readOnly
              ? m.writes_allow()
              : m.writes_lock()}
            class={[
              "btn btn-sm shrink-0 gap-2",
              workspace.readOnly ? "btn-ghost" : "btn-soft btn-warning",
            ]}
          >
            <Icon
              icon={workspace.readOnly ? "lucide:lock" : "lucide:pencil"}
              class="size-4"
            />
            {workspace.readOnly ? m.read_only() : m.writes_on()}
          </button>
        </header>

        <div class="relative min-h-0 flex-1">
          {#if mqtt}
            <div class="absolute inset-0 flex flex-col">
              <MqttView {view} onblocked={() => ask()} />
            </div>
          {:else if view === "chart" && workspace.browse.result}
            <div
              in:arrive={{ from: "right" }}
              out:depart={{ to: "right" }}
              class="absolute inset-0 flex flex-col pt-3"
            >
              <Lazy
                load={() => import("@gpql/ui/data/ResultChart.svelte")}
                props={{
                  columns: workspace.browse.result.columns,
                  rows: workspace.browse.result.rows,
                  labels: chartLabels(),
                }}
              />
            </div>
          {:else}
            <div
              in:arrive={{ from: "left" }}
              out:depart={{ to: "left" }}
              class="absolute inset-0 flex flex-col"
            >
              <ResultGrid
                result={workspace.browse.result}
                empty={table
                  ? workspace.nouns.emptyRows()
                  : workspace.nouns.pick()}
                types={workspace.columnTypes}
                {spot}
                needle={workspace.finding ? term.trim().toLowerCase() : ""}
                editable={!s3}
                onblocked={ask}
                browse={workspace.browse}
                actions={s3 ? objectActions : undefined}
                {status}
              />
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </Panel>
</TabLayout>

{#if asking}
  <ConfirmDialog
    title={m.writes_ask()}
    body={m.writes_ask_hint()}
    confirm={m.writes_allow()}
    cancel={m.cancel()}
    icon="lucide:pencil"
    tone="warning"
    onconfirm={allowWrites}
    oncancel={() => (asking = null)}
  />
{/if}

{#if removing}
  <ConfirmDialog
    title={m.s3_delete_ask({ key: removing.key })}
    body={m.s3_delete_hint({ bucket: removing.bucket })}
    confirm={m.s3_delete()}
    cancel={m.cancel()}
    onconfirm={removeObject}
    oncancel={() => (removing = null)}
  />
{/if}
