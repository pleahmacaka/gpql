<script lang="ts">
  import {
    ConfirmDialog,
    Dropdown,
    OptionRow,
    RowGroup,
    SettingRow,
  } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { workspace } from "$lib/session/workspace.svelte"

  let leaving = $state(false)

  function flipManual() {
    const writes = workspace.writes
    const pending = workspace.active?.openTransaction || writes.open

    if (writes.manual && pending) {
      leaving = true

      return
    }

    void writes.setManual(!writes.manual)
  }

  async function leaveManual() {
    leaving = false
    await workspace.writes.setManual(false)
  }
</script>

<RowGroup label={m.settings_guard()}>
  <OptionRow
    icon="lucide:lock"
    title={m.read_only()}
    detail={m.option_read_only_hint()}
    on={workspace.readOnly}
    onclick={() => workspace.setReadOnly(!workspace.readOnly)}
  />

  <SettingRow
    icon="lucide:timer"
    title={m.write_window()}
    detail={m.write_window_hint()}
  >
    <Dropdown
      label={m.write_window()}
      options={workspace.windows.map(minutes => ({
        value: String(minutes),
        label:
          minutes === 0
            ? m.write_window_never()
            : m.minutes({ count: minutes }),
      }))}
      value={String(workspace.writeWindow)}
      onpick={minutes => workspace.setWriteWindow(Number(minutes))}
    />
  </SettingRow>
</RowGroup>

<RowGroup label={m.settings_edits()}>
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
    onclick={flipManual}
  />
</RowGroup>

{#if leaving}
  <ConfirmDialog
    title={m.manual_off_title()}
    body={m.manual_off_body()}
    confirm={m.manual_off_confirm()}
    cancel={m.cancel()}
    icon="lucide:git-commit-horizontal"
    tone="warning"
    onconfirm={leaveManual}
    oncancel={() => (leaving = false)}
  />
{/if}
