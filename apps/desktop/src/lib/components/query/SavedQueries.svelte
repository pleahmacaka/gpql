<script lang="ts">
  import {
    arrive,
    ConfirmDialog,
    contextmenu,
    EmptyState,
    Icon,
    ListRow,
    Marker,
    Panel,
    Segmented,
  } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  import { selectFrom } from "./buffer"

  type Tab = "saved" | "history"

  type Step = [limit: number, size: number, unit: Intl.RelativeTimeFormatUnit]

  const STEPS: Step[] = [
    [60, 1, "second"],
    [3600, 60, "minute"],
    [86400, 3600, "hour"],
    [Number.POSITIVE_INFINITY, 86400, "day"],
  ]

  let tab = $state<Tab>("saved")
  let clearing = $state(false)

  let query = $derived(workspace.query)

  let ago = $derived(
    new Intl.RelativeTimeFormat(workspace.locale, {
      numeric: "auto",
      style: "narrow",
    }),
  )

  function when(seconds: number) {
    const gap = seconds - Math.floor(Date.now() / 1000)
    const [, size, unit] =
      STEPS.find(([limit]) => Math.abs(gap) < limit) ?? STEPS[3]

    return ago.format(Math.round(gap / size), unit)
  }

  function firstLine(sql: string) {
    return sql.trim().split("\n")[0]
  }

  function savedMenu(id: string, sql: string) {
    return [
      {
        label: m.history_open(),
        icon: "lucide:square-pen",
        run: () => query.load(id),
      },
      {
        label: m.menu_run(),
        icon: "lucide:play",
        run: async () => {
          query.load(id)
          await query.run()
        },
      },
      {
        label: m.menu_copy(),
        icon: "lucide:copy",
        run: () => navigator.clipboard.writeText(sql),
      },
      {
        label: m.menu_delete(),
        icon: "lucide:trash-2",
        danger: true,
        run: () => query.drop(id),
      },
    ]
  }

  async function runAgain(sql: string) {
    await query.replace(sql)
    await query.run()
  }
</script>

{#snippet placeholder()}
  <div
    role="status"
    aria-label={m.library_loading()}
    class="flex flex-col py-1"
  >
    {#each { length: 5 }, line (line)}
      <div class="flex h-12 items-center gap-3 px-4">
        <span class="skeleton size-4 shrink-0 opacity-60"></span>

        <span class="flex flex-1 flex-col gap-2">
          <span
            class={[
              "skeleton h-2 opacity-60",
              ["w-32", "w-40", "w-24", "w-36"][line % 4],
            ]}
          ></span>
          <span class="skeleton h-2 w-16 opacity-40"></span>
        </span>
      </div>
    {/each}
  </div>
{/snippet}

<Panel
  glass={false}
  label={m.library_label()}
  class={workspace.chat.dock === "panel"
    ? "w-64 shrink-0 max-lg:hidden"
    : "w-64 shrink-0"}
>
  <div
    class="flex h-11 shrink-0 items-center border-b border-base-content/10 px-2"
  >
    <div class="w-full">
      <Segmented
        small
        label={m.library_label()}
        value={tab}
        onpick={next => (tab = next === "history" ? "history" : "saved")}
        options={[
          { value: "saved", label: m.tab_saved(), icon: "lucide:bookmark" },
          { value: "history", label: m.tab_history(), icon: "lucide:history" },
        ]}
      />
    </div>
  </div>

  {#if workspace.favorites.length > 0 && tab === "saved"}
    <section
      class="flex max-h-40 shrink-0 flex-col border-b border-base-content/10"
    >
      <Marker as="h2" label={m.favorites()} class="h-8 shrink-0 px-3" />

      <div class="min-h-0 overflow-y-auto pb-1">
        {#each workspace.favorites as name (name)}
          <ListRow
            icon="lucide:star"
            title={name}
            trailing={null}
            onclick={() =>
              query.replace(
                selectFrom(
                  name,
                  workspace.session?.kind ?? "",
                  workspace.dialect,
                ),
              )}
          />
        {/each}
      </div>
    </section>
  {/if}

  <div class="flex h-9 shrink-0 items-center gap-2 pr-2 pl-3">
    <span
      class={[
        "min-w-0 flex-1 truncate text-xs text-base-content/70",
        "tabular-nums",
      ]}
    >
      {tab === "saved"
        ? m.saved_count({ count: query.saved.length })
        : m.history_count({ count: query.history.length })}
    </span>

    {#if tab === "saved"}
      <button
        type="button"
        onclick={() => query.keep()}
        disabled={query.sql.trim() === ""}
        class="btn btn-ghost btn-xs font-medium"
      >
        <Icon icon="lucide:bookmark-plus" class="size-4" />
        {m.keep_this()}
      </button>
    {:else}
      <button
        type="button"
        onclick={() => (clearing = true)}
        disabled={query.history.length === 0}
        class="btn btn-ghost btn-xs font-medium"
      >
        <Icon icon="lucide:trash-2" class="size-4" />
        {m.history_clear()}
      </button>
    {/if}
  </div>

  <div class="relative min-h-0 flex-1 overflow-hidden">
    {#key tab}
      <div
        in:arrive={{ from: tab === "history" ? "right" : "left", distance: 1 }}
        class="absolute inset-0 overflow-y-auto pb-2"
        style:scrollbar-gutter="stable"
      >
        {#if tab === "saved" && query.savedLoading}
          {@render placeholder()}
        {:else if tab === "saved"}
          {#each query.saved as entry (entry.id)}
            <div use:contextmenu={() => savedMenu(entry.id, entry.sql)}>
              <ListRow
                icon="lucide:file-code"
                title={entry.name}
                detail={entry.target}
                active={query.open === entry.id}
                trailing={null}
                dismissLabel={`${m.menu_delete()} ${entry.name}`}
                onclick={() => query.load(entry.id)}
                ondismiss={() => query.drop(entry.id)}
              />
            </div>
          {:else}
            <EmptyState
              art={null}
              title={m.saved_empty()}
              hint={m.saved_empty_hint()}
            />
          {/each}
        {:else if query.historyLoading}
          {@render placeholder()}
        {:else}
          {#each query.history as entry (entry.id)}
            <div
              class={[
                "group relative flex items-start transition-colors",
                "hover:bg-base-content/5",
              ]}
            >
              <button
                type="button"
                onclick={() => query.replace(entry.sql)}
                ondblclick={() => runAgain(entry.sql)}
                aria-label={`${m.history_open()} ${firstLine(entry.sql)}`}
                class={[
                  "flex min-w-0 flex-1 cursor-pointer items-start gap-3 py-2",
                  "pl-4 text-left outline-offset-0",
                ]}
              >
                <span
                  aria-hidden="true"
                  class={[
                    "mt-2 size-2 shrink-0",
                    entry.ok ? "bg-success" : "bg-error",
                  ]}
                ></span>

                <span class="flex min-w-0 flex-1 flex-col">
                  <span class="truncate text-sm">{firstLine(entry.sql)}</span>

                  <span
                    class="flex gap-2 text-xs text-base-content/70 tabular-nums"
                  >
                    <span>{when(entry.ranAt)}</span>
                    <span>{entry.millis} ms</span>
                  </span>
                </span>
              </button>

              <button
                type="button"
                aria-label={`${m.history_run()} ${firstLine(entry.sql)}`}
                onclick={() => runAgain(entry.sql)}
                disabled={query.busy}
                class={[
                  "btn btn-square btn-ghost btn-xs mt-2 mr-2 opacity-0",
                  "group-hover:opacity-100 focus-visible:opacity-100",
                ]}
              >
                <Icon icon="lucide:play" class="size-4" />
              </button>
            </div>
          {:else}
            <EmptyState art={null} title={m.nothing_run()} />
          {/each}
        {/if}
      </div>
    {/key}
  </div>
</Panel>

{#if clearing}
  <ConfirmDialog
    title={m.history_clear_ask()}
    confirm={m.history_clear()}
    cancel={m.cancel()}
    onconfirm={async () => {
      clearing = false
      await query.forgetHistory()
    }}
    oncancel={() => (clearing = false)}
  />
{/if}
