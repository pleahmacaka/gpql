<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Keycap, RowGroup } from "@gpql/ui"

  const groups = [
    {
      title: m.hint_app(),
      keys: [
        { label: m.key_palette(), keys: ["ctrl", "k"] },
        { label: m.action_new(), keys: ["ctrl", "n"] },
        { label: m.action_quick(), keys: ["ctrl", "l"] },
        { label: m.recent_connections(), keys: ["ctrl", "h"] },
        { label: m.erd_new(), keys: ["ctrl", "shift", "d"] },
        { label: m.erd_open(), keys: ["ctrl", "o"] },
        { label: m.action_open_file(), keys: ["ctrl", "shift", "o"] },
        { label: m.key_find(), keys: ["ctrl", "f"] },
        { label: m.key_tabs(), keys: ["1", "2", "3"] },
        { label: m.key_settings(), keys: ["ctrl", ","] },
        { label: m.key_agent(), keys: ["ctrl", "j"] },
        { label: m.key_orb(), keys: ["ctrl", "space"] },
        { label: m.key_close(), keys: ["escape"] },
      ],
    },
    {
      title: m.tab_data(),
      keys: [
        { label: m.key_cells(), keys: ["↑", "↓", "←", "→"] },
        { label: m.key_edit(), keys: ["enter"] },
        { label: m.key_copy(), keys: ["ctrl", "c"] },
      ],
    },
    {
      title: m.tab_query(),
      keys: [
        { label: m.key_run(), keys: ["ctrl", "enter"] },
        { label: m.key_clear(), keys: ["ctrl", "u"] },
        { label: m.key_complete(), keys: ["ctrl", "space"] },
        { label: m.format_sql(), keys: ["shift", "alt", "f"] },
      ],
    },
    {
      title: m.tab_schema(),
      keys: [{ label: m.arrows_walk(), keys: ["↑", "↓", "←", "→"] }],
    },
  ]
</script>

{#each groups as group (group.title)}
  <RowGroup label={group.title}>
    {#each group.keys as row (row.label)}
      <div class="flex items-center gap-4 px-4 py-2">
        <span class="min-w-0 flex-1 truncate text-sm">{row.label}</span>
        <Keycap keys={row.keys} />
      </div>
    {/each}
  </RowGroup>
{/each}
