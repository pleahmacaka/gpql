<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window"

  import { Icon } from "@gpql/ui"

  const win = getCurrentWindow()

  const buttons = [
    { label: "Minimize", icon: "lucide:minus", act: () => win.minimize() },
    {
      label: "Maximize",
      icon: "lucide:square",
      act: () => win.toggleMaximize(),
    },
    { label: "Close", icon: "lucide:x", act: () => win.close() },
  ]
</script>

<div class="flex">
  {#each buttons as button (button.label)}
    <button
      type="button"
      aria-label={button.label}
      onclick={button.act}
      class="grid h-11 w-12 place-items-center text-base-content/70
        transition-colors hover:bg-base-300
        {button.label === 'Close' ? 'hover:bg-error hover:text-error-content' : ''}"
    >
      <Icon
        icon={button.icon}
        class={button.label === "Maximize" ? "size-3" : "size-4"}
      />
    </button>
  {/each}
</div>
