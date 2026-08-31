<script lang="ts">
  import Dropdown from "../controls/Dropdown.svelte"
  import Field from "../controls/Field.svelte"
  import { Icon } from "../icons"
  import type {
    BackendInfo,
    CredentialPreset,
    Probe,
    SessionDraft,
  } from "../types"

  type Props = {
    draft: SessionDraft
    backends: BackendInfo[]
    presets?: CredentialPreset[]
    databases?: string[]
    keyFiles?: string[]
    probe?: Probe
    readOnly?: boolean
    busy?: boolean
    onconnect?: () => void
    onkeep?: () => void
    ontoggleReadOnly?: () => void
    onbrowse?: () => void
    onbrowseKey?: () => void
    tunnelled?: boolean
    localPortTaken?: boolean
    alias?: string
    onalias?: (value: string) => void
    labels?: Partial<Record<
      | "database"
      | "credentials"
      | "typed"
      | "readOnly"
      | "readOnlyHint"
      | "connect"
      | "save"
      | "browse"
      | "tlsAuto"
      | "tlsVerify"
      | "tlsRequire"
      | "tlsOff"
      | "tunnel"
      | "tunnelHost"
      | "tunnelUser"
      | "tunnelKey"
      | "tunnelPort"
      | "tunnelLocal"
      | "tunnelPicks"
      | "tunnelBusyPort"
      | "alias"
      | "aliasHint"
      | "viaHop"
      | "tunnelPassword"
      | "tunnelPassphrase",
      string
    >>
  }

  let {
    draft = $bindable(),
    backends,
    presets = [],
    databases = [],
    keyFiles = [],
    probe = { tone: "idle", text: "" },
    readOnly = true,
    busy = false,
    onconnect,
    onkeep,
    ontoggleReadOnly,
    onbrowse,
    onbrowseKey,
    tunnelled = false,
    localPortTaken = false,
    alias = "",
    onalias,
    labels = {},
  }: Props = $props()

  // the draft carries the hop so it saves and reloads with the connection
  let hop = $derived(
    (draft.tunnel ??= {
      host: "",
      port: "",
      user: "",
      password: "",
      keyPath: "",
      passphrase: "",
      localPort: "",
    }),
  )

  let hopOpen = $derived(hop.host !== "")

  // with a jump host in front, the host and port above stop meaning "from here"
  let hopping = $derived(tunnelled && hop.host.trim() !== "")

  let words = $derived({
    database: labels.database ?? "Database",
    credentials: labels.credentials ?? "Credentials",
    typed: labels.typed ?? "Typed by hand",
    readOnly: labels.readOnly ?? "Read only",
    readOnlyHint: labels.readOnlyHint ?? "the server refuses every write",
    connect: labels.connect ?? "Connect",
    save: labels.save ?? "Save",
    browse: labels.browse ?? "Browse",
    tlsAuto: labels.tlsAuto ?? "Automatic",
    tlsVerify: labels.tlsVerify ?? "Verify certificate",
    tlsRequire: labels.tlsRequire ?? "Encrypt only",
    tlsOff: labels.tlsOff ?? "Off",
    tunnel: labels.tunnel ?? "SSH tunnel",
    tunnelHost: labels.tunnelHost ?? "Jump host",
    tunnelUser: labels.tunnelUser ?? "SSH user",
    tunnelKey: labels.tunnelKey ?? "Private key",
    tunnelPort: labels.tunnelPort ?? "SSH port",
    tunnelLocal: labels.tunnelLocal ?? "Local port",
    tunnelPicks: labels.tunnelPicks ?? "left empty, GPQL picks a free one",
    tunnelBusyPort: labels.tunnelBusyPort ?? "something else already has it",
    alias: labels.alias ?? "Alias",
    aliasHint: labels.aliasHint ?? "shown instead of the database name",
    viaHop: labels.viaHop ?? "from the jump host",
    tunnelPassword: labels.tunnelPassword ?? "SSH password",
    tunnelPassphrase: labels.tunnelPassphrase ?? "Key passphrase",
  })

  let backend = $derived(
    backends.find(entry => entry.id === draft.kind) ?? backends[0],
  )

  let users = $derived([
    ...new Set(presets.map(entry => entry.user).filter(name => name !== "")),
  ])

  let matching = $derived(
    presets.filter(entry => entry.user === draft.user && entry.password !== ""),
  )

  const dot: Record<Probe["tone"], string> = {
    good: "bg-success",
    bad: "bg-error",
    busy: "animate-pulse bg-primary",
    idle: "bg-base-content/20",
  }

  function pickBackend(id: string) {
    const next = backends.find(entry => entry.id === id)

    draft.kind = id

    if (next && next.port !== "") {
      draft.port = next.port
    }

    for (const field of next?.fields ?? []) {
      if (draft[field.key] === undefined) {
        draft[field.key] = ""
      }
    }
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault()
      onconnect?.()
    }
  }
</script>

<div class="space-y-2">
  {#if onalias}
    <Field
      label={words.alias}
      placeholder={words.aliasHint}
      value={alias}
      oninput={value => onalias?.(value)}
      onkeydown={keys}
    />
  {/if}

  <div class="block rounded-field bg-base-200 px-3 pt-2 pb-2">
    <span class="block text-xs text-base-content/45">{words.database}</span>

    <Dropdown
      wide
      value={draft.kind}
      options={backends.map(entry => ({
        value: entry.id,
        label: entry.wip ? `${entry.label} (WIP)` : entry.label,
      }))}
      onpick={pickBackend}
    />
  </div>

  {#each backend?.fields ?? [] as field (field.key)}
    {#if field.key === "port"}
      <div class="hidden"></div>
    {:else if field.key === "host"}
      <div class="flex gap-2">
        <div class="flex-1">
          <Field
            label={hopping ? `${field.label} (${words.viaHop})` : field.label}
            placeholder={hopping ? "127.0.0.1" : field.placeholder}
            bind:value={draft.host}
            onkeydown={keys}
          />
        </div>

        {#if backend?.fields.some(entry => entry.key === "port")}
          <div class="w-24">
            <Field label="Port" bind:value={draft.port} onkeydown={keys} />
          </div>
        {/if}
      </div>
    {:else if field.key === "path" && onbrowse}
      <div class="flex gap-2">
        <div class="flex-1">
          <Field
            label={field.label}
            placeholder={field.placeholder}
            value={String(draft.path ?? "")}
            oninput={value => (draft.path = value)}
            onkeydown={keys}
          />
        </div>

        <button
          type="button"
          onclick={() => onbrowse?.()}
          class="flex items-center gap-2 rounded-field bg-base-200 px-3 text-sm
            transition-colors hover:bg-base-300"
        >
          <Icon icon="lucide:folder-open" class="size-4 text-base-content/45" />
          {words.browse}
        </button>
      </div>
    {:else if field.key === "database"}
      <div class="flex gap-2">
        <div class="flex-1">
          <Field
            label={field.label}
            placeholder={field.placeholder}
            value={String(draft.database ?? "")}
            oninput={value => (draft.database = value)}
            onkeydown={keys}
          />
        </div>

        {#if databases.length > 0}
          <div class="flex items-center rounded-field bg-base-200 px-3 text-sm">
            <Dropdown
              value={String(draft.database ?? "")}
              options={databases.map(name => ({ value: name, label: name }))}
              onpick={name => (draft.database = name)}
            />
          </div>
        {/if}
      </div>
    {:else if field.key === "tls"}
      <div class="flex items-center gap-3 rounded-field bg-base-200 px-3 py-2">
        <span class="flex-1 text-sm">{field.label}</span>

        <Dropdown
          value={String(draft.tls ?? "")}
          options={[
            { value: "", label: words.tlsAuto },
            { value: "verify-full", label: words.tlsVerify },
            { value: "require", label: words.tlsRequire },
            { value: "disable", label: words.tlsOff },
          ]}
          onpick={next => (draft.tls = next)}
        />
      </div>
    {:else if field.key === "user"}
      <div class="flex gap-2">
        <div class="flex-1">
          <Field
            label={field.label}
            placeholder={field.placeholder}
            value={String(draft.user ?? "")}
            oninput={value => (draft.user = value)}
            onkeydown={keys}
          />
        </div>

        {#if users.length > 0}
          <div class="flex items-center rounded-field bg-base-200 px-3 text-sm">
            <Dropdown
              value={String(draft.user ?? "")}
              options={users.map(name => ({ value: name, label: name }))}
              onpick={name => (draft.user = name)}
            />
          </div>
        {/if}
      </div>
    {:else if field.key === "password"}
      <div class="flex gap-2">
        <div class="flex-1">
          <Field
            label={field.label}
            placeholder={field.placeholder}
            type="password"
            value={String(draft.password ?? "")}
            oninput={value => (draft.password = value)}
            onkeydown={keys}
          />
        </div>

        {#if matching.length > 0}
          <div
            class="flex items-center rounded-field bg-base-200 px-3 text-sm"
          >
            <Dropdown
              value=""
              options={[
                { value: "", label: words.typed },
                ...matching.map(entry => ({
                  value: entry.name,
                  label: entry.name,
                })),
              ]}
              onpick={name => {
                const picked = presets.find(entry => entry.name === name)

                draft.password = picked?.password ?? ""
              }}
            />
          </div>
        {/if}
      </div>
    {:else}
      <Field
        label={hopping && field.key === "url"
          ? `${field.label} (${words.viaHop})`
          : field.label}
        placeholder={field.placeholder}
        type={field.secret ? "password" : "text"}
        value={String(draft[field.key] ?? "")}
        oninput={value => (draft[field.key] = value)}
        onkeydown={keys}
      />
    {/if}
  {/each}

  {#if tunnelled}
    <details
      class="rounded-field bg-base-100 px-3 py-2 hairline"
      open={hopOpen}
    >
      <summary class="cursor-pointer text-xs text-base-content/60">
        {words.tunnel}{hop.host ? ` · ${hop.host}` : ""}
      </summary>

      <div class="space-y-2 pt-2">
        <div class="flex gap-2">
          <div class="flex-1">
            <Field
              label={words.tunnelHost}
              placeholder="jump.example.com"
              value={hop.host}
              oninput={value => (hop.host = value)}
            />
          </div>

          <div class="w-24">
            <Field
              label={words.tunnelPort}
              placeholder="22"
              value={hop.port}
              oninput={value => (hop.port = value)}
            />
          </div>
        </div>

        <Field
          label={words.tunnelUser}
          value={hop.user}
          oninput={value => (hop.user = value)}
        />

        <Field
          label={words.tunnelKey}
          placeholder="~/.ssh/id_ed25519"
          value={hop.keyPath}
          oninput={value => (hop.keyPath = value)}
        />

        {#if keyFiles.length > 0 || onbrowseKey}
          <div class="flex items-center gap-2">
            {#if keyFiles.length > 0}
              <div
                class="flex flex-1 items-center rounded-field bg-base-200 px-3
                  py-2 text-sm"
              >
                <Dropdown
                  wide
                  value={hop.keyPath}
                  options={keyFiles.map(path => ({
                    value: path,
                    label: path.split(/[\\/]/).pop() ?? path,
                  }))}
                  onpick={path => (hop.keyPath = path)}
                />
              </div>
            {/if}

            {#if onbrowseKey}
              <button
                type="button"
                onclick={() => onbrowseKey?.()}
                class="rounded-field bg-base-200 px-3 py-2 text-xs
                  hover:bg-base-300"
              >
                {words.browse}
              </button>
            {/if}
          </div>
        {/if}

        <Field
          label={hop.keyPath ? words.tunnelPassphrase : words.tunnelPassword}
          type="password"
          value={hop.keyPath ? hop.passphrase : hop.password}
          oninput={value =>
            hop.keyPath ? (hop.passphrase = value) : (hop.password = value)}
        />

        <div class="flex gap-2">
          <div class="w-28">
            <Field
              label={words.tunnelLocal}
              placeholder="auto"
              value={hop.localPort}
              oninput={value => (hop.localPort = value)}
            />
          </div>

          <p
            class="flex flex-1 items-center text-xs {localPortTaken
              ? 'text-error'
              : 'text-base-content/40'}"
          >
            {localPortTaken ? words.tunnelBusyPort : words.tunnelPicks}
          </p>
        </div>
      </div>
    </details>
  {/if}

  {#if probe.text}
    <p class="flex items-start gap-2 px-1 pt-1 text-xs">
      {#if probe.tone === "good"}
        <Icon
          icon={backend?.icon ?? "lucide:database"}
          class="size-4 shrink-0 text-success"
        />
      {:else}
        <span
          class="mt-2 size-2 shrink-0 rounded-selector {dot[probe.tone]}"
        ></span>
      {/if}

      <span
        class="line-clamp-2 break-keep {probe.tone === 'bad'
          ? 'text-error'
          : 'text-base-content/45'}"
        title={probe.text}
      >
        {probe.text}
      </span>
    </p>
  {/if}

  <div class="flex items-center gap-3 rounded-field bg-primary/10 px-3 py-3">
    <Icon icon="lucide:lock" class="size-4 text-accent" />

    <div class="flex-1">
      <p class="text-sm text-primary">{words.readOnly}</p>
      <p class="text-xs text-base-content/45">{words.readOnlyHint}</p>
    </div>

    <input
      type="checkbox"
      aria-label={words.readOnly}
      class="toggle toggle-primary toggle-sm"
      checked={readOnly}
      onchange={() => ontoggleReadOnly?.()}
    />
  </div>

  <div class="flex gap-2">
    {#if onkeep}
      <button
        type="button"
        onclick={() => onkeep?.()}
        class="btn btn-sm h-9 flex-1 rounded-field border-0 bg-base-200
          font-normal shadow-none hover:bg-base-300"
      >
        {words.save}
      </button>
    {/if}

    <button
      type="button"
      onclick={() => onconnect?.()}
      disabled={busy}
      class="btn btn-primary btn-sm h-9 flex-1 rounded-field border-0
        font-normal shadow-none"
    >
      {words.connect}
    </button>
  </div>
</div>
