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

  type Props = { seed?: SessionConfig | null; onsaved?: () => void }

  let { seed = null, onsaved }: Props = $props()

  let config = $state<SessionConfig>(
    untrack(() => (seed ? { ...seed } : blankConfig())),
  )
  let probe = $state<Probe>({
    tone: "idle",
    text: m.probe_idle(),
  })

  let running: Fiber.RuntimeFiber<void, never> | null = null

  let catalogue = $state<string[]>([])
  let keyFiles = $state<string[]>([])
  let localPortTaken = $state(false)
  let alias = $state(
    untrack(
      () =>
        workspace.recents.find(entry => entry.url === workspace.editing)
          ?.alias ?? "",
    ),
  )

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

  // a jump host can carry anything that dials a server, whether the address
  // arrives as a host and port or inside a url
  let overHost = $derived(
    (backend?.fields ?? []).some(
      field => field.key === "host" || field.key === "url",
    ),
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
    api.run(api.sshKeys()).then(found => (keyFiles = found))
  })

  $effect(() => {
    const asked = Number(config.tunnel?.localPort ?? "")

    if (!Number.isInteger(asked) || asked < 1 || asked > 65535) {
      localPortTaken = false

      return
    }

    const timer = setTimeout(async () => {
      localPortTaken = !(await api.run(api.portFree(asked)))
    }, 400)

    return () => clearTimeout(timer)
  })

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

  async function pickKey() {
    const picked = await open({ multiple: false, directory: false })

    if (typeof picked === "string" && config.tunnel) {
      config.tunnel.keyPath = picked
    }
  }

  async function keep() {
    try {
      const url = await workspace.keepConnection(config)

      await workspace.renameRecent(url, alias)
      onsaved?.()
    } catch (failure) {
      probe = { tone: "bad", text: friendly(String(failure)) }
    }
  }

  async function start() {
    try {
      await workspace.open(config)
      await workspace.renameRecent(api.describe(config), alias)
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
  onkeep={keep}
  ontoggleReadOnly={() => workspace.toggle("readOnly")}
  onbrowse={pickFile}
  onbrowseKey={pickKey}
  tunnelled={overHost}
  databases={catalogue}
  {keyFiles}
  {localPortTaken}
  {alias}
  onalias={next => (alias = next)}
  labels={{
    database: m.field_database(),
    credentials: m.field_credentials(),
    typed: m.credentials_typed(),
    readOnly: m.read_only(),
    readOnlyHint: m.read_only_hint(),
    connect: m.connect(),
    save: m.save_connection(),
    browse: m.browse(),
    tlsAuto: m.tls_auto(),
    tlsVerify: m.tls_verify(),
    tlsRequire: m.tls_require(),
    tlsOff: m.tls_off(),
    tunnel: m.tunnel(),
    tunnelHost: m.tunnel_host(),
    tunnelUser: m.tunnel_user(),
    tunnelKey: m.tunnel_key(),
    tunnelPort: m.tunnel_port(),
    tunnelLocal: m.tunnel_local(),
    tunnelPicks: m.tunnel_picks(),
    tunnelBusyPort: m.tunnel_busy_port(),
    alias: m.field_alias(),
    aliasHint: m.alias_hint(),
    viaHop: m.via_hop(),
    tunnelPassword: m.tunnel_password(),
    tunnelPassphrase: m.tunnel_passphrase(),
  }}
/>
