<script lang="ts">
  import * as m from "$lib/paraglide/messages"
  import { getLocale, locales, setLocale } from "$lib/paraglide/runtime"

  import { OptionRow as OptionRow } from "@gpql/ui"
  import { workspace } from "$lib/session/workspace.svelte"
</script>

<label class="mb-2 flex items-center gap-3 rounded-field bg-base-200 px-3 py-2">
  <span class="flex-1 text-sm">{m.language()}</span>

  <select
    value={getLocale()}
    onchange={event => setLocale(event.currentTarget.value as "en" | "ko")}
    class="cursor-pointer bg-transparent text-sm outline-none"
  >
    {#each locales as locale (locale)}
      <option value={locale}>{locale === "ko" ? "한국어" : "English"}</option>
    {/each}
  </select>
</label>

<div class="space-y-2">
  <OptionRow
    icon="lucide:moon"
    title={m.option_dark()}
    detail={m.option_dark_hint()}
    on={workspace.dark}
    onclick={() => workspace.toggle("dark")}
  />

  <OptionRow
    icon="lucide:rows-3"
    title={m.option_compact()}
    detail={m.option_compact_hint()}
    on={workspace.compact}
    onclick={() => workspace.toggle("compact")}
  />

  <OptionRow
    icon="lucide:lock"
    title={m.read_only()}
    detail={m.option_read_only_hint()}
    on={workspace.readOnly}
    onclick={() => workspace.toggle("readOnly")}
  />

  <OptionRow
    icon="lucide:layers"
    title={m.option_acrylic()}
    detail={m.option_acrylic_hint()}
    on={workspace.acrylic}
    onclick={() => workspace.toggle("acrylic")}
  />

  {#if workspace.acrylic}
    <label class="flex items-center gap-3 rounded-field bg-base-200 px-3 py-2">
      <span class="text-xs text-base-content/45">{m.option_texture()}</span>

      <input
        type="range"
        min="0"
        max="100"
        value={workspace.texture}
        oninput={event =>
          workspace.setTexture(Number(event.currentTarget.value))}
        class="range range-primary range-xs flex-1"
      />

      <span class="w-8 text-right text-xs text-base-content/45">
        {workspace.texture}
      </span>
    </label>
  {/if}
</div>
