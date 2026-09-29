<script lang="ts">
  import ListRow from "@gpql/ui/controls/ListRow.svelte"
  import { Icon } from "@gpql/ui/icons/index.ts"
  import { calm } from "@gpql/ui/motion/index.ts"
  import SessionCard from "@gpql/ui/session/SessionCard.svelte"
  import type { SessionDraft } from "@gpql/ui/types.ts"
  import { onDestroy } from "svelte"

  import { backends, type Found, found } from "../sample"

  type Props = { active: boolean; onnext?: () => void }

  let { active, onnext }: Props = $props()

  const blank = (): SessionDraft => ({
    kind: "postgres",
    host: "",
    port: "5432",
    user: "",
    password: "",
    database: "",
    path: "",
    url: "",
    token: "",
    tls: "",
    tunnel: {
      host: "",
      port: "",
      user: "",
      password: "",
      keyPath: "",
      passphrase: "",
      localPort: "",
    },
  })

  const draftOf = (entry: Found): SessionDraft => ({
    ...blank(),
    kind: entry.kind,
    host: entry.host,
    port: entry.port,
    user: entry.user,
    database: entry.kind === "mqtt" ? "#" : entry.database,
    url: entry.kind === "neo4j" ? `neo4j://${entry.host}:${entry.port}` : "",
  })

  let picked = $state.raw(found[0])
  let draft = $state(draftOf(found[0]))
  let readOnly = $state(false)
  let played = $state(false)
  let scanning = $state(false)
  let busy = $state(false)

  let timers: ReturnType<typeof setTimeout>[] = []

  const label = (kind: string) =>
    backends.find(entry => entry.id === kind)?.label ?? kind

  const icon = (kind: string) =>
    backends.find(entry => entry.id === kind)?.icon ?? "lucide:database"

  $effect(() => {
    if (!active || played) {
      return
    }

    played = true

    if (calm()) {
      return
    }

    scanning = true
    timers.push(setTimeout(() => (scanning = false), 900))
  })

  onDestroy(() => timers.forEach(clearTimeout))

  function choose(entry: Found) {
    picked = entry
    draft = draftOf(entry)
  }

  function connect() {
    busy = true
    timers.push(
      setTimeout(
        () => {
          busy = false
          onnext?.()
        },
        calm() ? 0 : 520,
      ),
    )
  }
</script>

<div class="grid h-full grid-cols-2 items-start gap-2 p-2">
  <section
    aria-labelledby="quick-title"
    class="rounded-box bg-base-100 p-4 lift"
  >
    <header class="flex items-center gap-2 pb-3">
      <h3 id="quick-title" class="flex-1 text-sm font-medium">
        Quick connect
      </h3>

      <span
        class={[
          "flex items-center gap-2 text-xs tabular-nums",
          "text-base-content/70",
        ]}
      >
        <Icon
          icon="lucide:radar"
          class={["size-4 text-primary", scanning && "animate-spin"]}
        />
        {found.length} found
      </span>
    </header>

    <div class={["space-y-1", played && "scan"]}>
      {#each found as entry, index (entry.kind + entry.host + entry.port)}
        <div style:animation-delay="{index * 110}ms">
          <ListRow
            icon={entry.login ? icon(entry.kind) : "lucide:lock"}
            title={entry.database || `${label(entry.kind)} on ${entry.port}`}
            detail={entry.detail || `${entry.user}@${entry.host}:${entry.port}`}
            trailing={entry.login ? "lucide:arrow-right" : "lucide:pencil"}
            active={entry === picked}
            onclick={() => choose(entry)}
          />
        </div>
      {/each}
    </div>
  </section>

  <section
    aria-labelledby="session-title"
    class="max-h-full overflow-y-auto rounded-box bg-base-100 p-4 lift"
  >
    <h3 id="session-title" class="pb-3 text-sm font-medium">New session</h3>

    <SessionCard
      bind:draft
      {backends}
      {readOnly}
      {busy}
      tunnelled
      probe={picked.answer
        ? { tone: "good", text: picked.answer }
        : { tone: "idle", text: "checked as soon as you type" }}
      ontoggleReadOnly={() => (readOnly = !readOnly)}
      onconnect={connect}
    />
  </section>
</div>

<style>
  .scan > div {
    animation: found 420ms cubic-bezier(0.2, 0.8, 0.2, 1) backwards;
  }

  @keyframes found {
    from {
      opacity: 0;
      translate: -0.75rem 0;
    }
  }
</style>
