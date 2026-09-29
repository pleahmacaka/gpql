<script lang="ts">
  import { Dropdown, OptionRow, RowGroup, SettingRow } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { locales } from "$lib/paraglide/runtime"
  import {
    type Scheme,
    schemes,
    workspace,
  } from "$lib/session/workspace.svelte"

  const NAMES: Record<string, string> = {
    en: "English",
    ko: "한국어",
    ja: "日本語",
    zh: "中文",
  }

  const SCHEMES: Record<Scheme, () => string> = {
    system: m.theme_system,
    light: m.theme_light,
    dark: m.theme_dark,
  }

  let languages = $derived(
    locales.map(locale => ({ value: locale, label: NAMES[locale] ?? locale })),
  )

  let themes = $derived(
    schemes.map(scheme => ({ value: scheme, label: SCHEMES[scheme]() })),
  )
</script>

<RowGroup label={m.settings_display()}>
  <SettingRow
    icon="lucide:languages"
    title={m.language()}
    detail={m.language_hint()}
  >
    <Dropdown
      label={m.language()}
      options={languages}
      value={workspace.locale}
      onpick={next => workspace.speak(next)}
    />
  </SettingRow>

  <SettingRow
    icon="lucide:sun-moon"
    title={m.option_theme()}
    detail={m.option_theme_hint()}
  >
    <Dropdown
      label={m.option_theme()}
      options={themes}
      value={workspace.scheme}
      onpick={next => workspace.setScheme(next)}
    />
  </SettingRow>

  <OptionRow
    icon="lucide:rows-3"
    title={m.option_compact()}
    detail={m.option_compact_hint()}
    on={workspace.compact}
    onclick={() => workspace.toggle("compact")}
  />

  <OptionRow
    icon="lucide:map"
    title={m.option_minimap()}
    detail={m.option_minimap_hint()}
    on={workspace.minimap}
    onclick={() => workspace.toggle("minimap")}
  />

  <OptionRow
    icon="lucide:wand-sparkles"
    title={m.option_motion()}
    detail={m.option_motion_hint()}
    on={workspace.motion}
    onclick={() => workspace.toggle("motion")}
  />

  <SettingRow
    icon="lucide:message-circle"
    title={m.option_orb()}
    detail={m.option_orb_hint()}
  >
    <Dropdown
      label={m.option_orb()}
      options={[
        { value: "left", label: m.orb_left() },
        { value: "center", label: m.orb_center() },
        { value: "right", label: m.orb_right() },
      ]}
      value={workspace.chat.side}
      onpick={side => workspace.setOrbSide(side)}
    />
  </SettingRow>
</RowGroup>

<RowGroup label={m.settings_window()}>
  <OptionRow
    icon="lucide:layers"
    title={m.option_acrylic()}
    detail={m.option_acrylic_hint()}
    on={workspace.acrylic}
    onclick={() => workspace.toggle("acrylic")}
  />

  <label class="flex items-center gap-4 px-4 py-3">
    <span class="min-w-0 flex-1">
      <span class="block text-sm">{m.option_texture()}</span>
      <span class="block text-xs text-base-content/70">
        {m.option_texture_hint()}
      </span>
    </span>

    <input
      type="range"
      min="0"
      max="100"
      value={workspace.texture}
      oninput={event => workspace.setTexture(Number(event.currentTarget.value))}
      class="range range-primary range-xs w-40"
    />

    <span class="w-8 text-right text-xs text-base-content/70 tabular-nums">
      {workspace.texture}
    </span>
  </label>
</RowGroup>
