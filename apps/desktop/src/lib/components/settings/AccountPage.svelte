<script lang="ts">
  import { ConfirmDialog, Icon, RowGroup } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import * as api from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import { site, wipeCloud } from "$lib/sync/client"

  type Scope = "local" | "cloud"

  let note = $state("")
  let busy = $state(false)
  let asking = $state<Scope | null>(null)

  async function attempt(work: () => Promise<string>) {
    busy = true
    note = m.probe_checking()

    try {
      note = await work()
    } catch (failure) {
      note = String(failure)
    } finally {
      busy = false
    }
  }

  const signIn = () =>
    attempt(async () => {
      await api.run(api.signIn(site))
      await workspace.refreshAccount()

      return ""
    })

  const syncNow = () => attempt(() => workspace.syncNow())

  async function signOut() {
    await api.run(api.forgetAccount())
    await workspace.refreshAccount()
    note = ""
  }

  function wipe() {
    const scope = asking

    asking = null

    if (scope) {
      void attempt(() =>
        scope === "local" ? workspace.wipeLocal() : wipeCloud(),
      )
    }
  }

  type Wipe = { scope: Scope; title: string; hint: string; icon: string }

  const SCOPES: Wipe[] = [
    {
      scope: "local",
      title: m.reset_local(),
      hint: m.reset_local_hint(),
      icon: "lucide:hard-drive",
    },
    {
      scope: "cloud",
      title: m.reset_cloud(),
      hint: m.reset_cloud_hint(),
      icon: "lucide:cloud-off",
    },
  ]
</script>

<RowGroup label={m.settings_sync()}>
  <div class="flex items-center gap-4 px-4 py-4">
    <span
      class={[
        "grid size-10 shrink-0 place-items-center",
        workspace.signedIn
          ? "bg-primary/15 text-primary"
          : "bg-base-content/5 text-base-content/70",
      ]}
    >
      <Icon icon="lucide:user-round" class="size-5" />
    </span>

    <div class="min-w-0 flex-1">
      <p class="text-sm font-medium">
        {workspace.signedIn ? m.account_signed_in() : m.account_signed_out()}
      </p>
      <p aria-live="polite" class="text-xs wrap-anywhere text-base-content/70">
        {note || (workspace.signedIn ? m.sync_note() : m.sync_needs_login())}
      </p>
    </div>

    {#if workspace.signedIn}
      <button
        type="button"
        onclick={signOut}
        class="btn btn-ghost btn-sm font-medium hover:text-error"
      >
        {m.sign_out()}
      </button>

      <button
        type="button"
        onclick={syncNow}
        disabled={busy}
        class="btn btn-primary btn-sm font-medium"
      >
        <Icon
          icon="lucide:refresh-cw"
          class={["size-4", busy && "animate-spin"]}
        />
        {m.sync_now()}
      </button>
    {:else}
      <button
        type="button"
        onclick={signIn}
        disabled={busy}
        class="btn btn-primary btn-sm font-medium"
      >
        <Icon
          icon={busy ? "lucide:loader-circle" : "lucide:log-in"}
          class={["size-4", busy && "animate-spin"]}
        />
        {m.sign_in()}
      </button>
    {/if}
  </div>
</RowGroup>

<RowGroup label={m.settings_data()}>
  {#each SCOPES as entry (entry.scope)}
    <div class="flex items-center gap-4 px-4 py-3">
      <Icon icon={entry.icon} class="size-4 shrink-0 text-base-content/70" />

      <div class="min-w-0 flex-1">
        <p class="text-sm">{entry.title}</p>
        <p class="text-xs text-base-content/70">{entry.hint}</p>
      </div>

      <button
        type="button"
        onclick={() => (asking = entry.scope)}
        disabled={entry.scope === "cloud" && !workspace.signedIn}
        class="btn btn-sm btn-soft btn-error font-medium"
      >
        {m.reset()}
      </button>
    </div>
  {/each}
</RowGroup>

{#if asking}
  <ConfirmDialog
    title={asking === "local" ? m.reset_local_ask() : m.reset_cloud_ask()}
    body={asking === "local" ? m.reset_local_hint() : m.reset_cloud_hint()}
    confirm={m.reset()}
    cancel={m.cancel()}
    icon={asking === "local" ? "lucide:hard-drive" : "lucide:cloud-off"}
    onconfirm={wipe}
    oncancel={() => (asking = null)}
  />
{/if}
