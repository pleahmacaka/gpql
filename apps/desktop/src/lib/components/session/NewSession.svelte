<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { SessionCard } from "@gpql/ui"
  import { Effect, Fiber } from "effect"
  import { untrack } from "svelte"

  import { blankConfig, check } from "$lib/session/commands"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Probe, SessionConfig } from "$lib/types"

  type Props = { seed?: SessionConfig | null }

  let { seed = null }: Props = $props()

  let config = $state<SessionConfig>(
    untrack(() => (seed ? { ...seed } : blankConfig())),
  )
  let probe = $state<Probe>({
    tone: "idle",
    text: m.probe_idle(),
  })

  let running: Fiber.RuntimeFiber<void, never> | null = null

  let backend = $derived(
    workspace.catalog.find(entry => entry.id === config.kind),
  )

  let ready = $derived(
    (backend?.fields ?? [])
      .filter(field => !field.secret && field.key !== "port")
      .every(field => String(config[field.key] ?? "").trim() !== ""),
  )

  $effect(() => {
    void [
      config.kind,
      config.host,
      config.port,
      config.user,
      config.password,
      config.database,
      config.path,
    ]

    if (!ready) {
      stop()
      probe = { tone: "idle", text: m.probe_idle() }

      return
    }

    verify()

    return stop
  })

  function stop() {
    if (running) {
      Effect.runFork(Fiber.interrupt(running))
      running = null
    }
  }

  function verify() {
    stop()

    const snapshot = { ...config }

    running = Effect.runFork(
      Effect.gen(function* () {
        yield* Effect.sleep(700)

        probe = { tone: "busy", text: m.probe_checking() }

        const answer = yield* check(snapshot)

        probe = { tone: "good", text: answer }
      }).pipe(
        Effect.catchAll(failure =>
          Effect.sync(() => {
            probe = { tone: "bad", text: failure.message }
          }),
        ),
      ),
    )
  }

  async function start() {
    try {
      await workspace.open(config)
    } catch (failure) {
      probe = { tone: "bad", text: String(failure) }
    }
  }
</script>

<SessionCard
  bind:draft={config}
  backends={workspace.catalog}
  presets={workspace.presets}
  {probe}
  readOnly={workspace.readOnly}
  busy={workspace.busy}
  onconnect={start}
  ontoggleReadOnly={() => workspace.toggle("readOnly")}
  labels={{
    database: m.field_database(),
    credentials: m.field_credentials(),
    typed: m.credentials_typed(),
    readOnly: m.read_only(),
    readOnlyHint: m.read_only_hint(),
    connect: m.connect(),
    keys: m.keys_hint(),
  }}
/>
