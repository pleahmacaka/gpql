<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener"

  import { field, Icon, ListRow, mark, RowGroup } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { latestRelease, run } from "$lib/session/commands"
  import type { ReleaseCheck } from "$lib/types"

  const REPO = "https://github.com/pleahmacaka/gpql"
  const SITE = "https://gpql.dev"

  let version = $state(__GPQL_VERSION__)
  let checking = $state(false)
  let found = $state<ReleaseCheck | null>(null)
  let failed = $state(false)
  const signature = mark({ start: performance.now() / 1000 })

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

  let status = $derived(
    checking
      ? m.update_checking()
      : failed
        ? m.update_failed()
        : found
          ? found.fresh
            ? m.update_found({ version: found.latest })
            : m.update_latest()
          : m.update_check(),
  )
</script>

<div class="flex items-center gap-6">
  <canvas
    aria-hidden="true"
    class="size-32 shrink-0"
    {@attach field(signature, { cell: 0.375 })}
  ></canvas>

  <div class="flex min-w-0 flex-col gap-2">
    <h3 class="text-2xl font-extrabold tracking-tight">GPQL</h3>

    <p class="flex items-center gap-2 text-sm text-base-content/70">
      {m.about_version({ version })}

      {#if import.meta.env.DEV}
        <span class="badge badge-sm badge-soft badge-warning gap-1">
          <Icon icon="lucide:hammer" class="size-3" />
          Dev
        </span>
      {/if}
    </p>

    <div class="flex items-center gap-2 pt-2">
      <button
        type="button"
        onclick={check}
        disabled={checking}
        class="btn btn-soft btn-sm font-medium"
      >
        <Icon
          icon="lucide:refresh-cw"
          class={["size-4", checking && "animate-spin"]}
        />
        {status}
      </button>

      {#if found?.fresh}
        <button
          type="button"
          onclick={() => openUrl(found?.link ?? REPO)}
          class="btn btn-primary btn-sm font-medium"
        >
          <Icon icon="lucide:download" class="size-4" />
          {m.update_get()}
        </button>
      {/if}
    </div>
  </div>
</div>

<RowGroup>
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
</RowGroup>
