<script lang="ts">
  import { SvelteFlowProvider } from "@xyflow/svelte"

  import {
    ContextMenu,
    Dropdown,
    Icon,
    SchemaBoard,
    board,
    relationCount,
  } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import type { ErdDocument } from "$lib/erd/document.svelte"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = { doc: ErdDocument }

  let { doc }: Props = $props()

  let table = $derived(doc.tables.find(entry => entry.name === doc.selected))

  let targets = $derived([
    { value: "", label: m.erd_no_link() },
    ...doc.tables
      .filter(entry => entry.name !== doc.selected)
      .flatMap(entry =>
        entry.columns.map(column => ({
          value: `${entry.name}.${column.name}`,
          label: `${entry.name}.${column.name}`,
        })),
      ),
  ])

  $effect(() => {
    board.selected = doc.selected
  })

  let menu = $state<{
    x: number
    y: number
    items: { label: string; icon?: string; danger?: boolean; run: () => void }[]
  } | null>(null)

  function openMenu(event: MouseEvent) {
    event.preventDefault()

    const spot = event.target as HTMLElement | null
    const node = spot?.closest<HTMLElement>(".svelte-flow__node")
    const name = node?.dataset.id ?? ""
    const table = doc.tables.find(entry => entry.name === name)

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: table
        ? [
            {
              label: m.erd_add_column(),
              icon: "lucide:plus",
              run: () => {
                doc.selected = table.name
                doc.addColumn(table.name)
              },
            },
            {
              label: m.erd_duplicate_table(),
              icon: "lucide:copy",
              run: () => doc.duplicateTable(table.name),
            },
            {
              label: m.menu_delete(),
              icon: "lucide:trash-2",
              danger: true,
              run: () => doc.removeTable(table.name),
            },
          ]
        : [
            {
              label: m.erd_add_table(),
              icon: "lucide:table-2",
              run: () => doc.addTable(),
            },
          ],
    }
  }
</script>

<div class="flex h-full gap-2 p-2">
  <aside class="flex w-64 shrink-0 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-center gap-2 px-3 pt-2 pb-1">
      <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{doc.name}</h2>

      <button
        type="button"
        aria-label={m.erd_add_table()}
        onclick={() => doc.addTable()}
        class="rounded-selector p-1 text-base-content/40 hover:text-base-content"
      >
        <Icon icon="lucide:plus" class="size-4" />
      </button>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
      {#if doc.tables.length === 0}
        <p class="px-1 py-6 text-center text-sm text-base-content/45">
          {m.erd_empty()}
        </p>
      {:else}
        {#each doc.tables as entry (entry.name)}
          <div class="group flex items-center gap-1">
            <button
              type="button"
              onclick={() => (doc.selected = entry.name)}
              class="min-w-0 flex-1 truncate rounded-field px-2 py-1.5 text-left
                text-sm transition-colors {doc.selected === entry.name
                ? 'bg-primary/10 text-primary'
                : 'hover:bg-base-200'}"
            >
              {entry.name}

              <span class="pl-1 text-xs text-base-content/40">
                {m.columns_count({ count: entry.columns.length })}
              </span>
            </button>

            <button
              type="button"
              aria-label={m.menu_delete()}
              onclick={() => doc.removeTable(entry.name)}
              class="rounded-selector p-1 text-base-content/30 opacity-0
                transition-opacity group-hover:opacity-100 hover:text-error"
            >
              <Icon icon="lucide:trash-2" class="size-3.5" />
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </aside>

  <section class="flex min-w-0 flex-1 flex-col rounded-box bg-base-100 lift">
    <header class="flex items-baseline gap-2 px-4 pt-2 pb-1">
      <h2 class="text-sm font-medium">ERD</h2>

      <span class="text-xs text-base-content/45">
        {m.tables_count({ count: doc.tables.length })}
      </span>

      <span class="text-xs text-base-content/45">
        {m.relations_count({ count: relationCount(doc.tables) })}
      </span>

      <span class="flex-1"></span>

      {#if doc.failure}
        <span class="truncate text-xs text-error" title={doc.failure}>
          {doc.failure}
        </span>
      {:else}
        <span class="truncate text-xs text-base-content/35" title={doc.path}>
          {doc.path}
        </span>
      {/if}

      <button
        type="button"
        onclick={() => workspace.closeErd()}
        class="rounded-field px-2 py-0.5 text-xs text-base-content/45
          hover:bg-base-200 hover:text-base-content"
      >
        {m.erd_close()}
      </button>
    </header>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="min-h-0 flex-1 overflow-hidden rounded-box"
      oncontextmenu={openMenu}
    >
      <SvelteFlowProvider>
        <SchemaBoard
          minimap={workspace.minimap}
          tables={doc.tables}
          dark={workspace.dark}
          labels={{
    auto: m.arrange_auto(),
    picked: m.arrange_picked(),
    group: m.group_make(),
    ungroup: m.group_drop(),
    warn: m.arrange_warn(),
    groupName: m.group_name(),
  }}
          onselect={name => (doc.selected = name)}
        />
      </SvelteFlowProvider>
    </div>

    {#if menu}
      <ContextMenu
        x={menu.x}
        y={menu.y}
        items={menu.items}
        onclose={() => (menu = null)}
      />
    {/if}
  </section>

  {#if table}
    <aside
      class="flex w-80 shrink-0 flex-col gap-2 overflow-y-auto rounded-box
        bg-base-100 p-3 lift"
    >
      <input
        value={table.name}
        onchange={event =>
          doc.renameTable(table.name, event.currentTarget.value)}
        spellcheck="false"
        class="w-full rounded-field bg-base-200 px-3 py-1.5 text-sm outline-none"
      />

      {#each table.columns as column, index (index)}
        <div class="space-y-1.5 rounded-field bg-base-200 p-2">
          <div class="flex items-center gap-1.5">
            <input
              value={column.name}
              onchange={event =>
                doc.updateColumn(table.name, index, {
                  name: event.currentTarget.value,
                })}
              spellcheck="false"
              class="min-w-0 flex-1 bg-transparent text-sm outline-none"
            />

            <input
              value={column.dataType}
              onchange={event =>
                doc.updateColumn(table.name, index, {
                  dataType: event.currentTarget.value,
                })}
              spellcheck="false"
              class="w-24 bg-transparent text-right text-xs
                text-base-content/45 outline-none"
            />

            <button
              type="button"
              aria-label={m.menu_delete()}
              onclick={() => doc.removeColumn(table.name, index)}
              class="rounded-selector p-1 text-base-content/30 hover:text-error"
            >
              <Icon icon="lucide:trash-2" class="size-3.5" />
            </button>
          </div>

          <div class="flex items-center gap-3 text-xs">
            <label class="flex cursor-pointer items-center gap-1.5">
              <input
                type="checkbox"
                checked={column.primaryKey}
                onchange={event =>
                  doc.updateColumn(table.name, index, {
                    primaryKey: event.currentTarget.checked,
                  })}
                class="checkbox checkbox-xs checkbox-primary"
              />
              {m.erd_primary_key()}
            </label>

            <label class="flex cursor-pointer items-center gap-1.5">
              <input
                type="checkbox"
                checked={column.required}
                onchange={event =>
                  doc.updateColumn(table.name, index, {
                    required: event.currentTarget.checked,
                  })}
                class="checkbox checkbox-xs checkbox-primary"
              />
              {m.erd_required()}
            </label>

            <span class="flex-1"></span>

            <Dropdown
              value={column.references ?? ""}
              options={targets}
              onpick={next =>
                doc.updateColumn(table.name, index, {
                  references: next === "" ? null : next,
                })}
            />
          </div>
        </div>
      {/each}

      <button
        type="button"
        onclick={() => doc.addColumn(table.name)}
        class="flex items-center justify-center gap-1.5 rounded-field
          bg-base-200 py-2 text-sm transition-colors hover:bg-base-300"
      >
        <Icon icon="lucide:plus" class="size-3.5" />
        {m.erd_add_column()}
      </button>
    </aside>
  {/if}
</div>
