<script lang="ts">
  import type { Snippet } from "svelte"

  import Dropdown from "../controls/Dropdown.svelte"
  import Field from "../controls/Field.svelte"
  import { Icon } from "../icons"
  import type {
    BackendInfo,
    CredentialPreset,
    Probe,
    SessionDraft,
    SessionHop,
  } from "../types"

  type Props = {
    draft: SessionDraft
    backends: BackendInfo[]
    presets?: CredentialPreset[]
    databases?: string[]
    listing?: boolean
    keyFiles?: string[]
    probe?: Probe
    probeHint?: string
    readOnly?: boolean
    busy?: boolean
    pinned?: boolean
    onconnect?: () => void
    onkeep?: () => void
    ontoggleReadOnly?: () => void
    onbrowse?: () => void
    oncreate?: () => void
    onbrowseKey?: () => void
    tunnelled?: boolean
    localPortTaken?: boolean
    alias?: string
    onalias?: (value: string) => void
    labels?: Partial<
      Record<
        | "database"
        | "credentials"
        | "typed"
        | "readOnly"
        | "readOnlyHint"
        | "writesHint"
        | "connect"
        | "save"
        | "browse"
        | "newFile"
        | "newFileHint"
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
        | "tunnelPassphrase"
        | "port"
        | "found"
        | "listing"
        | "presets"
        | "keys",
        string
      >
    >
  }

  let {
    draft = $bindable(),
    backends,
    presets = [],
    databases = [],
    listing = false,
    keyFiles = [],
    probe = { tone: "idle", text: "" },
    probeHint = "",
    readOnly = true,
    busy = false,
    pinned = false,
    onconnect,
    onkeep,
    ontoggleReadOnly,
    onbrowse,
    oncreate,
    onbrowseKey,
    tunnelled = false,
    localPortTaken = false,
    alias = "",
    onalias,
    labels = {},
  }: Props = $props()

  const NO_HOP: SessionHop = {
    host: "",
    port: "",
    user: "",
    password: "",
    keyPath: "",
    passphrase: "",
    localPort: "",
  }

  let hop = $derived(draft.tunnel ?? NO_HOP)

  function setHop(key: keyof SessionHop, value: string) {
    draft.tunnel = { ...hop, [key]: value }
  }

  let hopOpen = $derived(hop.host !== "")

  // with a jump host in front, the host and port above stop meaning "from here"
  let hopping = $derived(tunnelled && hop.host.trim() !== "")

  let words = $derived({
    database: labels.database ?? "Database",
    credentials: labels.credentials ?? "Credentials",
    typed: labels.typed ?? "Typed by hand",
    readOnly: labels.readOnly ?? "Read only",
    readOnlyHint: labels.readOnlyHint ?? "The server refuses every write",
    writesHint: labels.writesHint ?? "Edits you apply reach the database",
    connect: labels.connect ?? "Connect",
    save: labels.save ?? "Save",
    browse: labels.browse ?? "Browse",
    newFile: labels.newFile ?? "New file",
    newFileHint: labels.newFileHint ?? "Created when you connect",
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
    tunnelPicks: labels.tunnelPicks ?? "Left empty, GPQL picks a free one",
    tunnelBusyPort: labels.tunnelBusyPort ?? "Something else already has it",
    alias: labels.alias ?? "Alias",
    aliasHint: labels.aliasHint ?? "Shown instead of the database name",
    viaHop: labels.viaHop ?? "from the jump host",
    tunnelPassword: labels.tunnelPassword ?? "SSH password",
    tunnelPassphrase: labels.tunnelPassphrase ?? "Key passphrase",
    port: labels.port ?? "Port",
    found: labels.found ?? "Found on the server",
    listing: labels.listing ?? "Looking for databases",
    presets: labels.presets ?? "Credential presets",
    keys: labels.keys ?? "Keys in .ssh",
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

  const SQUARE: Record<Probe["tone"], string> = {
    good: "bg-success",
    bad: "bg-error",
    busy: "animate-pulse bg-primary",
    idle: "bg-base-content/40",
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
    if (event.key !== "Enter" || event.isComposing) {
      return
    }

    event.preventDefault()

    if (!busy) {
      onconnect?.()
    }
  }
</script>

{#snippet side(label: string, children: Snippet)}
  <div
    class={[
      "flex h-8 shrink-0 items-center border border-base-content/20",
      "bg-base-100",
    ]}
    title={label}
  >
    {@render children()}
  </div>
{/snippet}

<div class={["flex flex-col gap-3", pinned && "min-h-full"]}>
  <div class="flex flex-col gap-1">
    <span class="text-xs text-base-content/70">{words.database}</span>

    <Dropdown
      wide
      label={words.database}
      value={draft.kind}
      options={backends.map(entry => ({
        value: entry.id,
        label: entry.wip ? `${entry.label} (WIP)` : entry.label,
      }))}
      onpick={pickBackend}
    />
  </div>

  {#if onalias}
    <Field
      label={words.alias}
      placeholder={words.aliasHint}
      value={alias}
      oninput={value => onalias?.(value)}
      onkeydown={keys}
    />
  {/if}

  {#each backend?.fields ?? [] as field (field.key)}
    {#if field.key === "port"}
      <div class="hidden"></div>
    {:else if field.key === "host"}
      <div class="flex items-end gap-2">
        <div class="min-w-0 flex-1">
          <Field
            label={hopping ? `${field.label} (${words.viaHop})` : field.label}
            placeholder={hopping ? "127.0.0.1" : field.placeholder}
            bind:value={draft.host}
            onkeydown={keys}
          />
        </div>

        {#if backend?.fields.some(entry => entry.key === "port")}
          <div class="w-24">
            <Field
              label={words.port}
              bind:value={draft.port}
              onkeydown={keys}
            />
          </div>
        {/if}
      </div>
    {:else if field.key === "path" && onbrowse}
      <div class="flex flex-col gap-2">
        <div class="flex items-end gap-2">
          <div class="min-w-0 flex-1">
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
            class="btn btn-soft btn-sm font-medium"
          >
            <Icon icon="lucide:folder-open" class="size-4" />
            {words.browse}
          </button>

          {#if oncreate}
            <button
              type="button"
              onclick={() => oncreate?.()}
              class="btn btn-soft btn-sm font-medium"
            >
              <Icon icon="lucide:file-plus-2" class="size-4" />
              {words.newFile}
            </button>
          {/if}
        </div>

        {#if draft.create === true}
          <p class="flex items-center gap-2 text-xs text-base-content/70">
            <Icon icon="lucide:sparkle" class="size-3 text-primary" />
            {words.newFileHint}
          </p>
        {/if}
      </div>
    {:else if field.key === "database"}
      <div class="flex items-end gap-2">
        <div class="min-w-0 flex-1">
          <Field
            label={field.label}
            placeholder={field.placeholder}
            value={String(draft.database ?? "")}
            oninput={value => (draft.database = value)}
            onkeydown={keys}
          />
        </div>

        {#if databases.length > 0}
          {#snippet found()}
            <Dropdown
              label={words.found}
              value={String(draft.database ?? "")}
              options={databases.map(name => ({ value: name, label: name }))}
              onpick={name => (draft.database = name)}
            />
          {/snippet}

          {@render side(words.found, found)}
        {:else if listing}
          {#snippet pending()}
            <span role="status" class="grid w-8 place-items-center">
              <Icon
                icon="lucide:loader-circle"
                class="size-4 animate-spin text-base-content/70"
              />
              <span class="sr-only">{words.listing}</span>
            </span>
          {/snippet}

          {@render side(words.listing, pending)}
        {/if}
      </div>
    {:else if field.key === "tls"}
      <div class="flex flex-col gap-1">
        <span class="text-xs text-base-content/70">{field.label}</span>

        <Dropdown
          wide
          label={field.label}
          value={String(draft.tls || "prefer")}
          options={[
            { value: "prefer", label: words.tlsAuto },
            { value: "verify-full", label: words.tlsVerify },
            { value: "require", label: words.tlsRequire },
            { value: "disable", label: words.tlsOff },
          ]}
          onpick={next => (draft.tls = next)}
        />
      </div>
    {:else if field.key === "user"}
      <div class="flex items-end gap-2">
        <div class="min-w-0 flex-1">
          <Field
            label={field.label}
            placeholder={field.placeholder}
            value={String(draft.user ?? "")}
            oninput={value => (draft.user = value)}
            onkeydown={keys}
          />
        </div>

        {#if users.length > 0}
          {#snippet named()}
            <Dropdown
              label={words.presets}
              value={String(draft.user ?? "")}
              options={users.map(name => ({ value: name, label: name }))}
              onpick={name => (draft.user = name)}
            />
          {/snippet}

          {@render side(words.presets, named)}
        {/if}
      </div>
    {:else if field.key === "password"}
      <div class="flex items-end gap-2">
        <div class="min-w-0 flex-1">
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
          {#snippet kept()}
            <Dropdown
              label={words.credentials}
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
          {/snippet}

          {@render side(words.credentials, kept)}
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
    <details class="group border border-base-content/10" open={hopOpen}>
      <summary
        class={[
          "flex cursor-pointer list-none items-center gap-2 px-3 py-2 text-sm",
          "transition-colors marker:content-none hover:bg-base-content/5",
        ]}
      >
        <Icon icon="lucide:waypoints" class="size-4 text-base-content/70" />

        <span class="min-w-0 flex-1 truncate">
          {words.tunnel}{hop.host ? `, ${hop.host}` : ""}
        </span>

        <Icon
          icon="lucide:chevron-down"
          class={[
            "size-4 text-base-content/70 transition-transform",
            "group-open:rotate-180",
          ]}
        />
      </summary>

      <div
        class="flex flex-col gap-4 border-t border-base-content/10 px-3 py-4"
      >
        <div class="flex items-end gap-2">
          <div class="min-w-0 flex-1">
            <Field
              label={words.tunnelHost}
              placeholder="jump.example.com"
              value={hop.host}
              oninput={value => setHop("host", value)}
            />
          </div>

          <div class="w-24">
            <Field
              label={words.tunnelPort}
              placeholder="22"
              value={hop.port}
              oninput={value => setHop("port", value)}
            />
          </div>
        </div>

        <Field
          label={words.tunnelUser}
          value={hop.user}
          oninput={value => setHop("user", value)}
        />

        <div class="flex items-end gap-2">
          <div class="min-w-0 flex-1">
            <Field
              label={words.tunnelKey}
              placeholder="~/.ssh/id_ed25519"
              value={hop.keyPath}
              oninput={value => setHop("keyPath", value)}
            />
          </div>

          {#if keyFiles.length > 0}
            {#snippet held()}
              <Dropdown
                label={words.keys}
                value={hop.keyPath}
                options={keyFiles.map(path => ({
                  value: path,
                  label: path.split(/[\\/]/).pop() ?? path,
                }))}
                onpick={path => setHop("keyPath", path)}
              />
            {/snippet}

            {@render side(words.keys, held)}
          {/if}

          {#if onbrowseKey}
            <button
              type="button"
              onclick={() => onbrowseKey?.()}
              class="btn btn-soft btn-sm font-medium"
            >
              {words.browse}
            </button>
          {/if}
        </div>

        <Field
          label={hop.keyPath ? words.tunnelPassphrase : words.tunnelPassword}
          type="password"
          value={hop.keyPath ? hop.passphrase : hop.password}
          oninput={value =>
            setHop(hop.keyPath ? "passphrase" : "password", value)}
        />

        <div class="w-32">
          <Field
            label={words.tunnelLocal}
            placeholder="auto"
            value={hop.localPort}
            invalid={localPortTaken}
            hint={localPortTaken ? words.tunnelBusyPort : words.tunnelPicks}
            oninput={value => setHop("localPort", value)}
          />
        </div>
      </div>
    </details>
  {/if}

  <label
    class={[
      "flex cursor-pointer items-center gap-3 px-3 py-3 transition-colors",
      readOnly ? "bg-base-content/5" : "bg-warning/10",
    ]}
  >
    <Icon
      icon={readOnly ? "lucide:lock" : "lucide:pencil"}
      class={["size-4 shrink-0", readOnly ? "text-primary" : "text-warning"]}
    />

    <span class="min-w-0 flex-1">
      <span class="block text-sm">{words.readOnly}</span>
      <span class="block text-xs text-base-content/70">
        {readOnly ? words.readOnlyHint : words.writesHint}
      </span>
    </span>

    <input
      type="checkbox"
      role="switch"
      class="toggle toggle-sm toggle-primary"
      checked={readOnly}
      onchange={event => {
        event.currentTarget.checked = readOnly
        ontoggleReadOnly?.()
      }}
    />
  </label>

  <div
    class={[
      "flex flex-col gap-3",
      pinned && "floating sticky bottom-0 -mx-4 mt-auto border-t px-4 py-4",
      pinned && "border-base-content/10",
    ]}
  >
    {#if probe.text}
      <p class="flex min-h-8 items-start gap-2 text-sm" aria-live="polite">
        {#if probe.tone === "good"}
          <Icon
            icon={backend?.icon ?? "lucide:database"}
            class="size-4 shrink-0 text-success"
          />
        {:else}
          <span class={["mt-2 size-2 shrink-0", SQUARE[probe.tone]]}></span>
        {/if}

        <span class="flex min-w-0 flex-col gap-1">
          <span
            class={[
              "line-clamp-2 break-keep",
              probe.tone === "bad" ? "text-error" : "text-base-content/70",
            ]}
            title={probe.text}
          >
            {probe.text}
          </span>

          {#if probeHint}
            <span class="text-base-content/70">{probeHint}</span>
          {/if}
        </span>
      </p>
    {/if}

    <div class="flex gap-2">
      {#if onkeep}
        <button
          type="button"
          onclick={() => onkeep?.()}
          class="btn btn-soft btn-sm flex-1 font-medium"
        >
          {words.save}
        </button>
      {/if}

      <button
        type="button"
        onclick={() => onconnect?.()}
        disabled={busy}
        class="btn btn-primary btn-sm flex-1 font-medium"
      >
        {#if busy}
          <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
        {/if}
        {words.connect}
      </button>
    </div>
  </div>
</div>
