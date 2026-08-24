<script lang="ts">
  import * as m from "$lib/paraglide/messages"
  import { getLocale, locales, setLocale } from "$lib/paraglide/runtime"

  import { Dropdown, OptionRow as OptionRow } from "@gpql/ui"
  import { type Scheme, schemes, workspace } from "$lib/session/workspace.svelte"

  const label = (scheme: Scheme) =>
    scheme === "system"
      ? m.theme_system()
      : scheme === "light"
        ? m.theme_light()
        : m.theme_dark()

  let languages = $derived(
    locales.map(locale => ({
      value: locale,
      label: locale === "ko" ? "한국어" : "English",
    })),
  )

  let themes = $derived(
    schemes.map(scheme => ({ value: scheme, label: label(scheme) })),
  )
</script>

<div class="mb-2 flex items-center gap-3 rounded-field bg-base-200 px-3 py-2">
  <span class="flex-1 text-sm">{m.language()}</span>

  <Dropdown
    options={languages}
    value={getLocale()}
    onpick={next => setLocale(next as "en" | "ko")}
  />
</div>

<div class="mb-2 flex items-center gap-3 rounded-field bg-base-200 px-3 py-2">
  <span class="flex-1 text-sm">{m.option_theme()}</span>

  <Dropdown
    options={themes}
    value={workspace.scheme}
    onpick={next => workspace.setScheme(next as Scheme)}
  />
</div>

<div class="space-y-2">
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
