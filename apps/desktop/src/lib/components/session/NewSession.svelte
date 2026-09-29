<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { SessionCard } from "@gpql/ui"
  import { Effect, Fiber } from "effect"
  import { untrack } from "svelte"

  import * as api from "$lib/session/commands"
  import { blankConfig, check } from "$lib/session/commands"
  import { open, save } from "@tauri-apps/plugin-dialog"

  import { friendly, hint } from "$lib/session/errors"
  import { workspace } from "$lib/session/workspace.svelte"
  import type { Probe, SessionConfig } from "$lib/types"

  import { DATABASE_FILTERS, FILES } from "./launcher.svelte"

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
  let listing = 0

  let catalogue = $state<string[]>([])
  let listingNow = $state(false)
  let keyFiles = $state<string[]>([])
  let localPortTaken = $state(false)
  let alias = $state(
    untrack(
      () =>
        workspace.recents.find(entry => entry.url === workspace.editing)
          ?.alias ?? "",
    ),
  )

  const LABELS: Record<string, () => string> = {
    Host: m.field_host,
    Port: m.field_port,
    User: m.field_user,
    Password: m.field_password,
    Database: m.field_database,
    File: m.field_file,
    Endpoint: m.field_endpoint,
    Region: m.field_region,
    Bucket: m.field_bucket,
    "Topic filter": m.field_topic_filter,
    Organization: m.field_organization,
    "Access key": m.field_access_key,
    "Secret key": m.field_secret_key,
    Token: m.field_token,
    "API token": m.field_api_token,
  }

  let backends = $derived(
    workspace.catalog.map(entry => ({
      ...entry,
      fields: entry.fields.map(field => ({
        ...field,
        label: LABELS[field.label]?.() ?? field.label,
      })),
    })),
  )

  let probeHint = $derived(probe.tone === "bad" ? hint(probe.text) : "")

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

  let fresh = $derived(FILES[config.kind])

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
      config.url,
      JSON.stringify(config.tunnel),
    ]

    listing++

    if (!canList) {
      catalogue = []
      listingNow = false

      return
    }

    const snapshot = $state.snapshot(config)
    const timer = setTimeout(() => void loadDatabases(snapshot), 600)

    return () => clearTimeout(timer)
  })

  $effect(() => {
    const snapshot = $state.snapshot(config)

    if (config.create === true && fresh) {
      stop()
      probe = { tone: "idle", text: "" }

      return
    }

    if (!ready) {
      stop()
      probe = { tone: "idle", text: m.probe_idle() }

      return
    }

    verify(snapshot)

    return stop
  })

  function stop() {
    if (running) {
      Effect.runFork(Fiber.interrupt(running))
      running = null
    }
  }

  function verify(snapshot = $state.snapshot(config)) {
    stop()

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
    const ticket = listing

    listingNow = true

    try {
      const found = await api.run(api.databases(snapshot))

      if (ticket === listing) {
        catalogue = found
      }
    } catch (failure) {
      if (ticket === listing) {
        catalogue = []
        probe = { tone: "bad", text: friendly(String(failure)) }
      }
    } finally {
      if (ticket === listing) {
        listingNow = false
      }
    }
  }

  async function pickFile() {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: DATABASE_FILTERS,
    })

    if (typeof picked === "string") {
      config.create = false
      config.path = picked
    }
  }

  async function createFile() {
    const kind = fresh

    if (!kind) {
      return
    }

    const picked = await save({
      defaultPath: kind.name,
      filters: [{ name: "Database", extensions: kind.extensions }],
    })

    if (picked) {
      config.create = true
      config.path = picked
    }
  }

  async function pickKey() {
    const picked = await open({ multiple: false, directory: false })

    if (typeof picked === "string" && config.tunnel) {
      config.tunnel = { ...config.tunnel, keyPath: picked }
    }
  }

  const editing = untrack(() => workspace.editing !== null)

  async function name(url: string) {
    if (editing || alias.trim() !== "") {
      await workspace.renameRecent(url, alias)
    }
  }

  async function keep() {
    try {
      const url = await workspace.keepConnection(config)

      await name(url)
      onsaved?.()
    } catch (failure) {
      probe = { tone: "bad", text: friendly(String(failure)) }
    }
  }

  async function start() {
    try {
      await workspace.open(config)
      await name(api.describe(config))
    } catch (failure) {
      probe = { tone: "bad", text: friendly(String(failure)) }
    }
  }
</script>

<SessionCard
  bind:draft={config}
  {backends}
  presets={workspace.presets}
  {probe}
  {probeHint}
  readOnly={workspace.readOnly}
  busy={workspace.busy}
  pinned
  onconnect={start}
  onkeep={keep}
  ontoggleReadOnly={() => workspace.setReadOnly(!workspace.readOnly)}
  onbrowse={pickFile}
  oncreate={fresh ? createFile : undefined}
  onbrowseKey={pickKey}
  tunnelled={overHost}
  databases={catalogue}
  listing={listingNow}
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
    newFile: m.new_file(),
    newFileHint: m.new_file_hint(),
    writesHint: m.writes_ask_hint(),
    found: m.found_databases(),
    listing: m.databases_loading(),
    presets: m.settings_credentials(),
    keys: m.ssh_keys(),
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
    port: m.field_port(),
  }}
/>
