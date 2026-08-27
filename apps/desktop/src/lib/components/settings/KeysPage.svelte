<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Keycap } from "@gpql/ui"

  const groups = [
    {
      title: m.hint_app(),
      keys: [
        { label: m.key_palette(), keys: ["ctrl", "k"] },
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
      ],
    },
    {
      title: m.tab_schema(),
      keys: [{ label: m.arrows_walk(), keys: ["↑", "↓", "←", "→"] }],
    },
  ]
</script>

<p class="pb-4 text-xs text-base-content/45">{m.keys_note()}</p>

<div class="space-y-5">
  {#each groups as group (group.title)}
    <section>
      <h3 class="px-1 pb-2 text-xs text-base-content/45">{group.title}</h3>

      <div class="space-y-1">
        {#each group.keys as row (row.label)}
          <div
            class="flex items-center gap-3 rounded-field px-3 py-2
              hover:bg-base-200"
          >
            <span class="min-w-0 flex-1 truncate text-sm">{row.label}</span>

            <Keycap keys={row.keys} />
          </div>
        {/each}
      </div>
    </section>
  {/each}
</div>
