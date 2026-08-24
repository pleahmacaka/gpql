<script lang="ts">
  import * as m from "$lib/paraglide/messages"

  import { Icon, OptionRow, Segmented } from "@gpql/ui"

  import { workspace } from "$lib/session/workspace.svelte"

  type Props = { ondone: () => void }

  let { ondone }: Props = $props()

  let step = $state(0)

  const steps = $derived([m.step_look(), m.step_habits(), m.step_ready()])
</script>

<div class="flex h-full items-center justify-center p-6">
  <section class="w-lg max-w-full rounded-box bg-base-100 p-6 lift">
    <p class="text-xs text-base-content/45">{m.first_run_eyebrow()}</p>

    <h1 class="pt-1 text-lg font-medium">{m.first_run_title()}</h1>

    <p class="pt-1 text-sm text-base-content/55">
      {m.first_run_body()}
    </p>

    <div class="pt-5">
      <Segmented
        value={steps[step]}
        options={steps.map(label => ({ value: label, label }))}
      />
    </div>

    <div class="space-y-2 pt-4">
      {#if step === 0}
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
          onclick={() => workspace.toggle("readOnly")}
        />

        <OptionRow
          icon="lucide:radar"
          title={m.option_scan()}
          detail={m.option_scan_hint()}
          on={workspace.autoscan}
          onclick={() => workspace.toggle("autoscan")}
        />
      {:else}
        <div class="rounded-field bg-base-200 px-3 py-3 text-sm">
          <p class="flex items-center gap-2">
            <Icon icon="lucide:command" class="size-4 text-base-content/45" />
            {m.first_run_tip_keys()}
          </p>

          <p class="flex items-center gap-2 pt-2">
            <Icon icon="lucide:radar" class="size-4 text-base-content/45" />
            {m.first_run_tip_scan()}
          </p>

          <p class="flex items-center gap-2 pt-2">
            <Icon icon="lucide:lock" class="size-4 text-accent" />
            {m.first_run_tip_vault()}
          </p>
        </div>
      {/if}
    </div>

    <div class="flex items-center gap-2 pt-5">
      {#if step > 0}
        <button
          type="button"
          onclick={() => (step -= 1)}
          class="rounded-field bg-base-200 px-4 py-2 text-sm hover:bg-base-300"
        >
          {m.first_run_back()}
        </button>
      {/if}

      <span class="flex-1"></span>

      <button
        type="button"
        onclick={() => (step < steps.length - 1 ? (step += 1) : ondone())}
        class="rounded-field bg-primary px-4 py-2 text-sm text-primary-content
          hover:bg-primary/90"
      >
        {step < steps.length - 1 ? m.first_run_next() : m.first_run_start()}
      </button>
    </div>
  </section>
</div>
