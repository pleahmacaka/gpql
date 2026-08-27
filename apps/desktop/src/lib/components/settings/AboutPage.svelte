<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon, ListRow, Logo } from "@gpql/ui"
  import { latestRelease, run } from "$lib/session/commands"
  import type { ReleaseCheck } from "$lib/types"
  import { openUrl } from "@tauri-apps/plugin-opener"

  const REPO = "https://github.com/pleahmacaka/gpql"
  const SITE = "https://gpql.dev"

  let version = $state(__GPQL_VERSION__)
  let checking = $state(false)
  let found = $state<ReleaseCheck | null>(null)
  let failed = $state(false)

  async function check() {
    checking = true
    failed = false

    try {
      found = await run(latestRelease())
      version = found.current
    } catch {
      failed = true
    } finally {
      checking = false
    }
  }
</script>

<div class="flex flex-col items-center pt-4 pb-8 text-center">
  <Logo class="size-24" />

  <h2 class="pt-4 font-display text-xl font-medium">GPQL</h2>

  <p class="pt-1 text-xs text-base-content/45">
    {m.about_version({ version })}
  </p>

  <div class="flex items-center gap-2 pt-5">
    <button
      type="button"
      onclick={check}
      disabled={checking}
      class="flex items-center gap-1.5 rounded-field bg-base-200 px-3 py-1.5
        text-sm transition-colors hover:bg-base-300 disabled:opacity-60"
    >
      {#if checking}
        <Icon icon="lucide:loader-circle" class="size-3.5 animate-spin" />
      {:else}
        <Icon icon="lucide:refresh-cw" class="size-3.5" />
      {/if}
      {checking
        ? m.update_checking()
        : failed
          ? m.update_failed()
          : found
            ? found.fresh
              ? m.update_found({ version: found.latest })
              : m.update_latest()
            : m.update_check()}
    </button>

    {#if found?.fresh}
      <button
        type="button"
        onclick={() => openUrl(found?.link ?? REPO)}
        class="flex items-center gap-1.5 rounded-field bg-primary px-3 py-1.5
          text-sm text-primary-content transition-colors hover:bg-primary/90"
      >
        <Icon icon="lucide:download" class="size-3.5" />
        {m.update_get()}
      </button>
    {/if}
  </div>

</div>

<div class="space-y-1">
  <ListRow
    icon="lucide:arrow-up-right"
    title={m.about_site()}
    detail={SITE}
    onclick={() => openUrl(SITE)}
  />

  <ListRow
    icon="simple-icons:github"
    title={m.about_source()}
    detail={REPO}
    onclick={() => openUrl(REPO)}
  />
</div>
