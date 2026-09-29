<script lang="ts">
  import {
    Dropdown,
    field,
    Icon,
    Keycap,
    Marker,
    mark,
    OptionRow,
    Panel,
    SettingRow,
    scramble,
  } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"
  import { locales } from "$lib/paraglide/runtime"
  import { workspace } from "$lib/session/workspace.svelte"

  type Props = { ondone: () => void }

  let { ondone }: Props = $props()

  const NAMES: Record<string, string> = {
    en: "English",
    ko: "한국어",
    ja: "日本語",
    zh: "中文",
  }

  const signature = mark({ start: performance.now() / 1000, assemble: 2 })

  let step = $state(0)

  let steps = $derived([m.step_look(), m.step_habits(), m.step_ready()])
  let last = $derived(step === steps.length - 1)
</script>

<div class="grid h-full grid-cols-12 gap-6 p-6">
  <div aria-hidden="true" class="relative col-span-5 hidden lg:block">
    <canvas
      class="absolute inset-0 size-full"
      {@attach field(signature, { cell: 0.625 })}
    ></canvas>
  </div>

  <Panel
    class="col-span-12 min-h-0 lg:col-span-7"
    inner="min-h-0 flex-1 overflow-y-auto"
    label={m.first_run_title()}
  >
    <header class="flex flex-col gap-3 px-8 pt-8 pb-6">
      <Marker label={m.first_run_eyebrow()} />

      <h1
        use:scramble={{ duration: 800 }}
        class="text-3xl leading-tight font-bold tracking-tight text-balance"
      >
        {m.first_run_title()}
      </h1>

      <p class="text-sm leading-relaxed text-pretty text-base-content/80">
        {m.first_run_body()}
      </p>
    </header>

    <ol class="flex border-y border-base-content/10 bg-base-content/5 px-8">
      {#each steps as label, index (label)}
        <li class="flex-1">
          <button
            type="button"
            onclick={() => (step = index)}
            aria-current={index === step ? "step" : undefined}
            class={[
              "relative flex w-full cursor-pointer items-center gap-3 py-3",
              "text-left text-sm transition-colors",
              index === step
                ? "font-medium text-base-content"
                : "text-base-content/70 hover:text-base-content",
            ]}
          >
            <span
              aria-hidden="true"
              class={[
                "size-2 shrink-0",
                index < step
                  ? "bg-success"
                  : index === step
                    ? "bg-primary"
                    : "bg-base-content/30",
              ]}
            ></span>

            <span class="text-xs text-base-content/70 tabular-nums">
              {String(index + 1).padStart(2, "0")}
            </span>

            {label}

            {#if index === step}
              <span
                aria-hidden="true"
                class="absolute inset-x-0 bottom-0 border-b-2 border-primary"
              ></span>
            {/if}
          </button>
        </li>
      {/each}
    </ol>

    <div class="flex flex-col divide-y divide-base-content/10 px-4 py-4">
      {#if step === 0}
        <SettingRow
          icon="lucide:languages"
          title={m.language()}
          detail={m.language_hint()}
        >
          <Dropdown
            label={m.language()}
            options={locales.map(locale => ({
              value: locale,
              label: NAMES[locale] ?? locale,
            }))}
            value={workspace.locale}
            onpick={next => workspace.speak(next)}
          />
        </SettingRow>

        <OptionRow
          icon="lucide:moon"
          title={m.option_dark()}
          detail={m.option_dark_hint()}
          on={workspace.dark}
          onclick={() => workspace.toggle("dark")}
        />

        <OptionRow
          icon="lucide:layers"
          title={m.option_acrylic()}
          detail={m.option_acrylic_hint()}
          on={workspace.acrylic}
          onclick={() => workspace.toggle("acrylic")}
        />

        <OptionRow
          icon="lucide:rows-3"
          title={m.option_compact()}
          detail={m.option_compact_hint()}
          on={workspace.compact}
          onclick={() => workspace.toggle("compact")}
        />
      {:else if step === 1}
        <OptionRow
          icon="lucide:lock"
          title={m.read_only()}
          detail={m.first_run_readonly_hint()}
          on={workspace.readOnly}
          onclick={() => workspace.setReadOnly(!workspace.readOnly)}
        />

        <OptionRow
          icon="lucide:sparkles"
          title={m.ai_on()}
          detail={m.ai_on_hint()}
          on={workspace.ai}
          onclick={() => workspace.toggle("ai")}
        />

        <OptionRow
          icon="lucide:radar"
          title={m.option_scan()}
          detail={m.option_scan_hint()}
          on={workspace.autoscan}
          onclick={() => workspace.toggle("autoscan")}
        />
      {:else}
        <div class="flex items-center gap-4 px-4 py-3 text-sm">
          <Icon icon="lucide:command" class="size-4 text-primary" />
          <span class="flex-1">{m.first_run_tip_keys()}</span>
          <Keycap keys={["ctrl", "k"]} />
        </div>

        <div class="flex items-center gap-4 px-4 py-3 text-sm">
          <Icon icon="lucide:radar" class="size-4 text-primary" />
          <span class="flex-1">{m.first_run_tip_scan()}</span>
        </div>

        <div class="flex items-center gap-4 px-4 py-3 text-sm">
          <Icon icon="lucide:shield-check" class="size-4 text-primary" />
          <span class="flex-1">{m.first_run_tip_vault()}</span>
        </div>
      {/if}
    </div>

    <footer
      class={[
        "mt-auto flex items-center gap-2 border-t border-base-content/10 px-8",
        "py-4",
      ]}
    >
      {#if step > 0}
        <button
          type="button"
          onclick={() => (step -= 1)}
          class="btn btn-soft btn-sm font-medium"
        >
          {m.first_run_back()}
        </button>
      {/if}

      <span class="flex-1"></span>

      <button
        type="button"
        onclick={() => (last ? ondone() : (step += 1))}
        class="btn btn-primary btn-sm font-medium"
      >
        {last ? m.first_run_start() : m.first_run_next()}
        <Icon icon="lucide:arrow-right" class="size-4" />
      </button>
    </footer>
  </Panel>
</div>
