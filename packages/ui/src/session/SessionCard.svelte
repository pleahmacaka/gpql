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
    probe?: Probe
    readOnly?: boolean
    busy?: boolean
    onconnect?: () => void
    onkeep?: () => void
    ontoggleReadOnly?: () => void
    onbrowse?: () => void
    labels?: Partial<Record<
      | "database"
      | "credentials"
      | "typed"
      | "readOnly"
      | "readOnlyHint"
      | "connect"
      | "save"
      | "keys"
      | "browse"
      | "tlsAuto"
      | "tlsVerify"
      | "tlsRequire"
      | "tlsOff",
      string
    >>
  }

  let {
    draft = $bindable(),
    backends,
    presets = [],
    databases = [],
    probe = { tone: "idle", text: "" },
    readOnly = true,
    busy = false,
    onconnect,
    onkeep,
    ontoggleReadOnly,
    onbrowse,
    labels = {},
  }: Props = $props()

  let words = $derived({
    database: labels.database ?? "Database",
    credentials: labels.credentials ?? "Credentials",
    typed: labels.typed ?? "Typed by hand",
    readOnly: labels.readOnly ?? "Read only",
    readOnlyHint: labels.readOnlyHint ?? "the server refuses every write",
    connect: labels.connect ?? "Connect",
    save: labels.save ?? "Save",
    keys: labels.keys ?? "tab moves, return connects",
    browse: labels.browse ?? "Browse",
    tlsAuto: labels.tlsAuto ?? "Automatic",
    tlsVerify: labels.tlsVerify ?? "Verify certificate",
    tlsRequire: labels.tlsRequire ?? "Encrypt only",
    tlsOff: labels.tlsOff ?? "Off",
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
  <div class="block rounded-field bg-base-200 px-3 pt-1.5 pb-1.5">
    <span class="block text-xs text-base-content/45">{words.database}</span>

    <Dropdown
      wide
      value={draft.kind}
      options={backends.map(entry => ({ value: entry.id, label: entry.label }))}
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
            label={field.label}
            placeholder={field.placeholder}
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
          class="flex items-center gap-1.5 rounded-field bg-base-200 px-3 text-sm
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
            suggestions={databases}
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
      <Field
        label={field.label}
        placeholder={field.placeholder}
        suggestions={users}
        value={String(draft.user ?? "")}
        oninput={value => (draft.user = value)}
        onkeydown={keys}
      />
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
        label={field.label}
        placeholder={field.placeholder}
        type={field.secret ? "password" : "text"}
        value={String(draft[field.key] ?? "")}
        oninput={value => (draft[field.key] = value)}
        onkeydown={keys}
      />
    {/if}
  {/each}

  {#if probe.text}
    <p class="flex items-center gap-2 px-1 pt-1 text-xs">
      <span class="size-1.5 shrink-0 rounded-selector {dot[probe.tone]}"></span>

      <span
        class="truncate {probe.tone === 'bad'
          ? 'text-error'
          : 'text-base-content/45'}"
        title={probe.text}
      >
        {probe.text}
      </span>
    </p>
  {/if}

  <div class="flex items-center gap-3 rounded-field bg-primary/10 px-3 py-2.5">
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

  <p class="px-1 text-xs text-base-content/40">{words.keys}</p>
</div>
