<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { SessionCard } from "@gpql/ui"
  import { Effect, Fiber } from "effect"
  import { untrack } from "svelte"

  import * as api from "$lib/session/commands"
  import { blankConfig, check } from "$lib/session/commands"
  import { open } from "@tauri-apps/plugin-dialog"

  import { friendly } from "$lib/session/errors"
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

  let catalogue = $state<string[]>([])

  let backend = $derived(
    workspace.catalog.find(entry => entry.id === config.kind),
  )

  let ready = $derived(
    (backend?.fields ?? [])
      .filter(
        field => !field.secret && field.key !== "port" && field.key !== "tls",
      )
      .every(field => String(config[field.key] ?? "").trim() !== ""),
  )

  let wantsDatabase = $derived(
    (backend?.fields ?? []).some(field => field.key === "database"),
  )

  let canList = $derived(
    wantsDatabase &&
      (String(config.token ?? "").trim() !== "" ||
        (String(config.host ?? "").trim() !== "" &&
          String(config.user ?? "").trim() !== "")),
  )

  $effect(() => {
    void [
      config.kind,
      config.host,
      config.port,
      config.user,
      config.password,
      config.token,
    ]

    if (!canList) {
      catalogue = []

      return
    }

    const snapshot = { ...config }
    const timer = setTimeout(() => void loadDatabases(snapshot), 600)

    return () => clearTimeout(timer)
  })

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
            probe = { tone: "bad", text: friendly(failure.message) }
          }),
        ),
      ),
    )
  }

  async function loadDatabases(snapshot: SessionConfig) {
    try {
      catalogue = await api.run(api.databases(snapshot))
    } catch (failure) {
      catalogue = []
      probe = { tone: "bad", text: friendly(String(failure)) }
    }
  }

  async function pickFile() {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: "Database",
          extensions: ["db", "sqlite", "sqlite3", "duckdb", "ddb"],
        },
      ],
    })

    if (typeof picked === "string") {
      config.path = picked
      verify()
    }
  }

  async function start() {
    try {
      await workspace.open(config)
    } catch (failure) {
      probe = { tone: "bad", text: friendly(String(failure)) }
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
  onbrowse={pickFile}
  databases={catalogue}
  labels={{
    database: m.field_database(),
    credentials: m.field_credentials(),
    typed: m.credentials_typed(),
    readOnly: m.read_only(),
    readOnlyHint: m.read_only_hint(),
    connect: m.connect(),
    keys: m.keys_hint(),
    browse: m.browse(),
    tlsAuto: m.tls_auto(),
    tlsVerify: m.tls_verify(),
    tlsRequire: m.tls_require(),
    tlsOff: m.tls_off(),
  }}
/>
