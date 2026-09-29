<script lang="ts">
  import {
    board,
    ConfirmDialog,
    Dropdown,
    EmptyState,
    Icon,
    ListRow,
    Marker,
    type MenuItem,
    Panel,
    pop,
    relationCount,
    rise,
    SchemaBoard,
    tooltip,
    veil,
  } from "@gpql/ui"
  import { SvelteFlowProvider } from "@xyflow/svelte"
  import { fade, scale } from "svelte/transition"

  import { boardWords } from "$lib/components/schema/words"
  import type { ErdDocument } from "$lib/erd/document.svelte"
  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  import { withHints } from "./refs"

  type Props = { doc: ErdDocument }

  let { doc }: Props = $props()

  const TYPES = [
    "text",
    "varchar(255)",
    "integer",
    "bigint",
    "serial",
    "uuid",
    "boolean",
    "numeric(12, 2)",
    "real",
    "date",
    "timestamp",
    "timestamptz",
    "json",
    "jsonb",
    "bytea",
  ]

  let removing = $state<string | null>(null)
  let clash = $state<{ table: string; column: number | null } | null>(null)
  let fresh = $state<{ table: boolean; column: number | null }>({
    table: false,
    column: null,
  })

  let table = $derived(doc.tables.find(entry => entry.name === doc.selected))
  let slot = $derived(table ? doc.tables.indexOf(table) : -1)
  let drawn = $derived(withHints(doc.tables))
  let unsaved = $derived(doc.dirty && (doc.untitled || doc.failure !== ""))

  let targets = $derived(
    [
      ...new Set([
        "",
        ...doc.tables
          .filter(entry => entry.name !== doc.selected)
          .flatMap(entry =>
            entry.columns.map(column => `${entry.name}.${column.name}`),
          ),
      ]),
    ].map(value => ({ value, label: value === "" ? m.erd_no_link() : value })),
  )

  board.reset()

  $effect(() => {
    board.selected = doc.selected
  })

  function saveKeys(event: KeyboardEvent) {
    if (event.isComposing || !event.ctrlKey || event.code !== "KeyS") {
      return
    }

    event.preventDefault()
    void (event.shiftKey ? doc.saveAs() : doc.save())
  }

  function addTable() {
    doc.addTable()
    fresh = { table: true, column: null }
  }

  function addColumn(name: string) {
    doc.selected = name
    doc.addColumn(name)

    const grown = doc.tables.find(entry => entry.name === name)

    fresh = { table: false, column: (grown?.columns.length ?? 1) - 1 }
  }

  function grabTable(node: HTMLInputElement) {
    if (fresh.table) {
      node.focus()
      node.select()
      fresh = { table: false, column: null }
    }
  }

  function grabColumn(index: number) {
    return (node: HTMLInputElement) => {
      if (fresh.column === index) {
        node.focus()
        node.select()
        fresh = { table: false, column: null }
      }
    }
  }

  function renameTable(input: HTMLInputElement, from: string) {
    const name = input.value.trim()
    const taken = doc.tables.some(
      entry => entry.name === name && entry.name !== from,
    )

    clash = taken ? { table: from, column: null } : null

    if (name === "" || taken) {
      input.value = from

      return
    }

    doc.renameTable(from, name)
  }

  function renameColumn(input: HTMLInputElement, index: number) {
    if (!table) {
      return
    }

    const name = input.value.trim()
    const taken = table.columns.some(
      (column, spot) => spot !== index && column.name === name,
    )

    clash = taken ? { table: table.name, column: index } : null

    if (name === "" || taken) {
      input.value = table.columns[index]?.name ?? ""

      return
    }

    doc.updateColumn(table.name, index, { name })
  }

  function retype(name: string, index: number, input: HTMLInputElement) {
    const dataType = input.value.trim() || "text"

    input.value = dataType
    doc.updateColumn(name, index, { dataType })
  }

  function remove() {
    if (removing !== null) {
      doc.removeTable(removing)
    }

    removing = null
  }

  function items(name: string | null): MenuItem[] {
    const found = doc.tables.find(entry => entry.name === name)

    if (!found) {
      return [
        { label: m.erd_add_table(), icon: "lucide:table-2", run: addTable },
      ]
    }

    return [
      {
        label: m.erd_add_column(),
        icon: "lucide:plus",
        run: () => addColumn(found.name),
      },
      {
        label: m.erd_duplicate_table(),
        icon: "lucide:copy",
        run: () => doc.duplicateTable(found.name),
      },
      {
        label: m.menu_delete(),
        icon: "lucide:trash-2",
        danger: true,
        run: () => (removing = found.name),
      },
    ]
  }
</script>

<svelte:window onkeydown={saveKeys} />

<datalist id="erd-types">
  {#each TYPES as type (type)}
    <option value={type}></option>
  {/each}
</datalist>

<div class="flex h-full gap-2 p-2">
  <div class="hidden w-56 shrink-0 lg:flex">
    <Panel glass={false} label={m.erd_tables()} class="min-w-0 flex-1">
      <header
        class={[
          "flex h-12 shrink-0 items-center gap-2 border-b",
          "border-base-content/10 pr-2 pl-4",
        ]}
      >
        <h2 class="min-w-0 flex-1 truncate text-sm font-semibold">
          {m.erd_tables()}
        </h2>

        <span class="text-xs text-base-content/70 tabular-nums">
          {doc.tables.length}
        </span>

        <button
          type="button"
          aria-label={m.erd_add_table()}
          use:tooltip={m.erd_add_table()}
          onclick={addTable}
          class="btn btn-square btn-ghost btn-sm"
        >
          <Icon icon="lucide:plus" class="size-4" />
        </button>
      </header>

      <div class="min-h-0 flex-1 overflow-y-auto py-1">
        {#each doc.tables as entry (entry.name)}
          <div in:rise>
            <ListRow
              icon="lucide:table-2"
              title={entry.name}
              detail={m.columns_count({ count: entry.columns.length })}
              active={doc.selected === entry.name}
              trailing={null}
              dismissLabel="{m.menu_delete()}, {entry.name}"
              onclick={() => (doc.selected = entry.name)}
              ondismiss={() => (removing = entry.name)}
            />
          </div>
        {:else}
          <p class="px-4 py-6 text-center text-xs text-base-content/70">
            {m.erd_empty_title()}
          </p>
        {/each}
      </div>
    </Panel>
  </div>

  <Panel glass={false} label={doc.name} class="min-w-0 flex-1">
    <div class="@container shrink-0 border-b border-base-content/10">
      <header class="flex h-12 items-center gap-2 pr-2 pl-4">
        <Icon icon="lucide:git-fork" class="size-4 shrink-0 text-primary" />

        <h2
          use:tooltip={doc.path || doc.name}
          class="min-w-0 truncate text-sm font-semibold"
        >
          {doc.name}
        </h2>

        {#if unsaved}
          <span
            in:scale={pop()}
            class="badge badge-sm badge-warning shrink-0 font-medium"
          >
            {m.erd_unsaved()}
          </span>
        {/if}

        <p
          class={[
            "hidden min-w-0 items-center gap-3 text-xs whitespace-nowrap",
            "text-base-content/70 tabular-nums @2xl:flex",
          ]}
        >
          <span>{m.tables_count({ count: doc.tables.length })}</span>
          <span>{m.relations_count({ count: relationCount(doc.tables) })}</span>
        </p>

        <span class="flex-1"></span>

        {#if doc.failure}
          <p
            use:tooltip={doc.failure}
            class="min-w-0 truncate text-xs text-error select-text"
          >
            {doc.failure}
          </p>
        {:else if !doc.untitled && !doc.dirty}
          <p
            in:fade={veil()}
            class={[
              "hidden shrink-0 items-center gap-1 text-xs",
              "text-base-content/70 @xl:flex",
            ]}
          >
            <Icon icon="lucide:check" class="size-3" />
            {m.erd_saved()}
          </p>
        {/if}

        <button
          type="button"
          onclick={() => doc.saveAs()}
          aria-keyshortcuts="Control+Shift+S"
          use:tooltip={"Ctrl+Shift+S"}
          class="btn btn-ghost btn-sm shrink-0 font-medium"
        >
          {m.erd_save_as()}
        </button>

        <button
          type="button"
          onclick={() => doc.save()}
          aria-keyshortcuts="Control+S"
          use:tooltip={"Ctrl+S"}
          class={[
            "btn btn-sm shrink-0 font-medium",
            unsaved ? "btn-primary" : "btn-soft",
          ]}
        >
          <Icon icon="lucide:save" class="size-4" />
          {m.erd_save()}
        </button>

        <button
          type="button"
          aria-label={m.erd_close()}
          use:tooltip={m.erd_close()}
          onclick={() => workspace.closeErd()}
          class="btn btn-square btn-ghost btn-sm shrink-0"
        >
          <Icon icon="lucide:x" class="size-4" />
        </button>
      </header>
    </div>

    <div class="relative min-h-0 flex-1 overflow-hidden">
      <SvelteFlowProvider>
        <SchemaBoard
          tables={drawn}
          dark={workspace.dark}
          minimap={workspace.minimap}
          labels={boardWords()}
          menuItems={items}
          onselect={name => (doc.selected = name)}
        />
      </SvelteFlowProvider>

      {#if doc.tables.length === 0}
        <div
          transition:fade={veil()}
          class="absolute inset-0 grid place-items-center bg-base-100"
        >
          <EmptyState
            art="graph"
            title={m.erd_empty_title()}
            hint={m.erd_empty_hint()}
          >
            <button
              type="button"
              onclick={addTable}
              class="btn btn-primary btn-sm font-medium"
            >
              <Icon icon="lucide:plus" class="size-4" />
              {m.erd_add_table()}
            </button>
          </EmptyState>
        </div>
      {/if}
    </div>
  </Panel>

  <Panel
    glass={false}
    label={table?.name ?? m.erd_pick()}
    class="w-72 shrink-0"
  >
    {#if table}
      {@const current = table}

      {#key slot}
        <div in:fade={veil()} class="flex min-h-0 flex-1 flex-col">
          <header
            class={[
              "flex h-12 shrink-0 items-center gap-1 border-b",
              "border-base-content/10 pr-2 pl-4",
            ]}
          >
            <Icon icon="lucide:table-2" class="size-4 shrink-0 text-primary" />

            <h2 class="min-w-0 flex-1 truncate pl-1 text-sm font-semibold">
              {current.name}
            </h2>

            <button
              type="button"
              aria-label={m.erd_duplicate_table()}
              use:tooltip={m.erd_duplicate_table()}
              onclick={() => doc.duplicateTable(current.name)}
              class="btn btn-square btn-ghost btn-sm"
            >
              <Icon icon="lucide:copy" class="size-4" />
            </button>

            <button
              type="button"
              aria-label="{m.menu_delete()}, {current.name}"
              use:tooltip={m.menu_delete()}
              onclick={() => (removing = current.name)}
              class="btn btn-square btn-ghost btn-sm hover:text-error"
            >
              <Icon icon="lucide:trash-2" class="size-4" />
            </button>
          </header>

          <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-4">
            <label class="flex flex-col gap-1">
              <span class="text-xs text-base-content/70">
                {m.erd_table_name()}
              </span>

              <input
                value={current.name}
                onchange={event =>
                  renameTable(event.currentTarget, current.name)}
                aria-invalid={clash?.table === current.name &&
                  clash.column === null}
                spellcheck="false"
                autocomplete="off"
                {@attach grabTable}
                class={[
                  "input input-sm w-full bg-base-100",
                  clash?.table === current.name &&
                    clash.column === null &&
                    "input-error",
                ]}
              />

              {#if clash?.table === current.name && clash.column === null}
                <span in:fade={veil()} class="text-xs text-error">
                  {m.erd_table_taken()}
                </span>
              {/if}
            </label>

            <label class="flex flex-col gap-1">
              <span class="text-xs text-base-content/70">
                {m.erd_table_note()}
              </span>

              <textarea
                value={current.note ?? ""}
                placeholder={m.erd_note_hint()}
                onchange={event =>
                  doc.updateTable(current.name, {
                    note: event.currentTarget.value.trim() || null,
                  })}
                rows="2"
                spellcheck="false"
                class={[
                  "textarea textarea-sm w-full resize-none bg-base-100",
                  "placeholder:text-base-content/60",
                ]}
              ></textarea>
            </label>

            <section class="flex flex-col gap-2">
              <div class="flex items-center gap-2">
                <Marker
                  as="h3"
                  label={m.erd_columns()}
                  class="min-w-0 flex-1"
                />

                <span class="text-xs text-base-content/70 tabular-nums">
                  {current.columns.length}
                </span>
              </div>

              <ul class="flex flex-col gap-2">
                {#each current.columns as column, index (index)}
                  {@const taken =
                    clash?.table === current.name && clash.column === index}

                  <li
                    in:rise
                    class="flex flex-col gap-2 bg-base-100 p-2 hairline"
                  >
                    <div class="flex items-center gap-1">
                      <input
                        value={column.name}
                        aria-label={m.erd_column_name()}
                        aria-invalid={taken}
                        onchange={event =>
                          renameColumn(event.currentTarget, index)}
                        spellcheck="false"
                        autocomplete="off"
                        {@attach grabColumn(index)}
                        class={[
                          "input input-sm min-w-0 flex-1 bg-base-100",
                          column.primaryKey && "font-semibold",
                          taken && "input-error",
                        ]}
                      />

                      <input
                        value={column.dataType}
                        aria-label={m.erd_column_type()}
                        list="erd-types"
                        onchange={event =>
                          retype(current.name, index, event.currentTarget)}
                        spellcheck="false"
                        autocomplete="off"
                        class="input input-sm w-24 bg-base-100 text-xs"
                      />

                      <button
                        type="button"
                        aria-label="{m.menu_delete()}, {column.name}"
                        use:tooltip={m.menu_delete()}
                        onclick={() => doc.removeColumn(current.name, index)}
                        class="btn btn-square btn-ghost btn-xs hover:text-error"
                      >
                        <Icon icon="lucide:x" class="size-4" />
                      </button>
                    </div>

                    {#if taken}
                      <span in:fade={veil()} class="text-xs text-error">
                        {m.erd_column_taken()}
                      </span>
                    {/if}

                    <div class="flex items-center gap-4 text-xs">
                      <label class="flex cursor-pointer items-center gap-2">
                        <input
                          type="checkbox"
                          checked={column.primaryKey}
                          onchange={event =>
                            doc.updateColumn(current.name, index, {
                              primaryKey: event.currentTarget.checked,
                            })}
                          class="checkbox checkbox-xs checkbox-primary"
                        />
                        {m.erd_primary_key()}
                      </label>

                      <label class="flex cursor-pointer items-center gap-2">
                        <input
                          type="checkbox"
                          checked={column.required}
                          onchange={event =>
                            doc.updateColumn(current.name, index, {
                              required: event.currentTarget.checked,
                            })}
                          class="checkbox checkbox-xs checkbox-primary"
                        />
                        {m.erd_required()}
                      </label>
                    </div>

                    <div class="flex items-center gap-2">
                      <Icon
                        icon="lucide:link-2"
                        class="size-4 shrink-0 text-base-content/70"
                      />

                      <Dropdown
                        wide
                        small
                        label={m.erd_references()}
                        search={m.find_tables()}
                        value={column.references ?? ""}
                        options={targets}
                        onpick={next =>
                          doc.updateColumn(current.name, index, {
                            references: next === "" ? null : next,
                          })}
                      />
                    </div>

                    <input
                      value={column.note ?? ""}
                      aria-label={m.erd_note()}
                      placeholder={m.erd_note_hint()}
                      onchange={event =>
                        doc.updateColumn(current.name, index, {
                          note: event.currentTarget.value.trim() || null,
                        })}
                      spellcheck="false"
                      autocomplete="off"
                      class={[
                        "input input-xs w-full bg-base-100 text-xs",
                        "placeholder:text-base-content/60",
                      ]}
                    />
                  </li>
                {/each}
              </ul>

              <button
                type="button"
                onclick={() => addColumn(current.name)}
                class="btn btn-soft btn-sm w-full font-medium"
              >
                <Icon icon="lucide:plus" class="size-4" />
                {m.erd_add_column()}
              </button>
            </section>
          </div>
        </div>
      {/key}
    {:else}
      <div in:fade={veil()} class="grid flex-1 place-items-center">
        <EmptyState art={null} title={m.erd_pick()} hint={m.erd_pick_hint()} />
      </div>
    {/if}
  </Panel>
</div>

{#if removing !== null}
  <ConfirmDialog
    title={m.erd_delete_ask({ name: removing })}
    body={m.erd_delete_hint()}
    confirm={m.menu_delete()}
    cancel={m.cancel()}
    onconfirm={remove}
    oncancel={() => (removing = null)}
  />
{/if}
