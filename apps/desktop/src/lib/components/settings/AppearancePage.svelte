<script lang="ts">
  import * as m from "$lib/paraglide/messages"
  import { locales } from "$lib/paraglide/runtime"

  import { Dropdown, OptionRow, SettingRow } from "@gpql/ui"
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
    value={workspace.locale}
    onpick={next => workspace.speak(next as "en" | "ko")}
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
    icon="lucide:map"
    title={m.option_minimap()}
    detail={m.option_minimap_hint()}
    on={workspace.minimap}
    onclick={() => workspace.toggle("minimap")}
  />

  <SettingRow
    icon="lucide:message-circle"
    title={m.option_orb()}
    detail={m.option_orb_hint()}
  >
    <Dropdown
      options={[
        { value: "left", label: m.orb_left() },
        { value: "center", label: m.orb_center() },
        { value: "right", label: m.orb_right() },
      ]}
      value={workspace.chat.side}
      onpick={side => workspace.setOrbSide(side)}
    />
  </SettingRow>

  <SettingRow
    icon="lucide:door-open"
    title={m.option_startup()}
    detail={m.option_startup_hint()}
  >
    <Dropdown
      options={[
        { value: "last", label: m.startup_last() },
        { value: "recent", label: m.startup_recent() },
      ]}
      value={workspace.startup}
      onpick={mode => workspace.setStartup(mode)}
    />
  </SettingRow>

  <SettingRow
    icon="lucide:timer"
    title={m.write_window()}
    detail={m.write_window_hint()}
  >
    <Dropdown
      options={workspace.windows.map(minutes => ({
        value: String(minutes),
        label: minutes === 0 ? m.write_window_never() : m.minutes({ count: minutes }),
      }))}
      value={String(workspace.writeWindow)}
      onpick={minutes => workspace.setWriteWindow(Number(minutes))}
    />
  </SettingRow>

  <OptionRow
    icon="lucide:file-pen-line"
    title={m.option_preview()}
    detail={m.option_preview_hint()}
    on={workspace.writes.preview}
    onclick={() => workspace.writes.setPreview(!workspace.writes.preview)}
  />

  <OptionRow
    icon="lucide:git-commit-horizontal"
    title={m.option_manual()}
    detail={workspace.writes.available
      ? m.option_manual_hint()
      : m.tx_unsupported()}
    on={workspace.writes.manual}
    onclick={() => workspace.writes.setManual(!workspace.writes.manual)}
  />

  <OptionRow
    icon="lucide:lock"
    title={m.read_only()}
    detail={m.option_read_only_hint()}
    on={workspace.readOnly}
    onclick={() => workspace.toggle("readOnly")}
  />

  <OptionRow
    icon="lucide:wand-sparkles"
    title={m.option_motion()}
    detail={m.option_motion_hint()}
    on={workspace.motion}
    onclick={() => workspace.toggle("motion")}
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
