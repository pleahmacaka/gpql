<script lang="ts">
  import Field from "./Field.svelte"
  import { Icon } from "./icons"
  import type {
    BackendInfo,
    CredentialPreset,
    Probe,
    SessionDraft,
  } from "./types"

  type Props = {
    draft: SessionDraft
    backends: BackendInfo[]
    presets?: CredentialPreset[]
    probe?: Probe
    readOnly?: boolean
    busy?: boolean
    onconnect?: () => void
    ontoggleReadOnly?: () => void
    labels?: Partial<Record<
      "database" | "credentials" | "typed" | "readOnly" | "readOnlyHint" | "connect" | "keys",
      string
    >>
  }

  let {
    draft = $bindable(),
    backends,
    presets = [],
    probe = { tone: "idle", text: "" },
    readOnly = true,
    busy = false,
    onconnect,
    ontoggleReadOnly,
    labels = {},
  }: Props = $props()

  let words = $derived({
    database: labels.database ?? "Database",
    credentials: labels.credentials ?? "Credentials",
    typed: labels.typed ?? "Typed by hand",
    readOnly: labels.readOnly ?? "Read only",
    readOnlyHint: labels.readOnlyHint ?? "the server refuses every write",
    connect: labels.connect ?? "Connect",
    keys: labels.keys ?? "tab moves, return connects",
  })

  let preset = $state("")

  let backend = $derived(
    backends.find(entry => entry.id === draft.kind) ?? backends[0],
  )

  let wantsCredentials = $derived(
    presets.length > 0 &&
      (backend?.fields.some(field => field.key === "user") ?? false),
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

  function applyPreset() {
    const picked = presets.find(entry => entry.name === preset)

    if (!picked) {
      return
    }

    draft.user = picked.user
    draft.password = picked.password
  }

  function keys(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault()
      onconnect?.()
    }
  }
</script>

<div class="space-y-2">
  <label class="block rounded-field bg-base-200 px-3 pt-1.5 pb-1.5">
    <span class="block text-xs text-base-content/45">{words.database}</span>

    <select
      value={draft.kind}
      onchange={event => pickBackend(event.currentTarget.value)}
      class="w-full cursor-pointer bg-transparent text-sm outline-none"
    >
      {#each backends as entry (entry.id)}
        <option value={entry.id}>{entry.label}</option>
      {/each}
    </select>
  </label>

  {#if wantsCredentials}
    <label class="block rounded-field bg-base-200 px-3 pt-1.5 pb-1.5">
      <span class="block text-xs text-base-content/45">{words.credentials}</span>

      <select
        bind:value={preset}
        onchange={applyPreset}
        class="w-full cursor-pointer bg-transparent text-sm outline-none"
      >
        <option value="">{words.typed}</option>

        {#each presets as entry (entry.name)}
          <option value={entry.name}>{entry.name}</option>
        {/each}
      </select>
    </label>
  {/if}

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

  <button
    type="button"
    onclick={() => onconnect?.()}
    disabled={busy}
    class="btn btn-primary btn-sm h-9 w-full rounded-field border-0 font-normal
      shadow-none"
  >
    {words.connect}
  </button>

  <p class="px-1 text-xs text-base-content/40">{words.keys}</p>
</div>
