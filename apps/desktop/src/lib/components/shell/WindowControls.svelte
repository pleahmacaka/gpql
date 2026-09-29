<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window"

  import { Icon } from "@gpql/ui"

  import * as m from "$lib/paraglide/messages"

  const win = getCurrentWindow()

  const buttons = [
    {
      label: m.window_minimize(),
      icon: "lucide:minus",
      act: () => win.minimize(),
      close: false,
    },
    {
      label: m.window_maximize(),
      icon: "lucide:square",
      act: () => win.toggleMaximize(),
      close: false,
    },
    {
      label: m.window_close(),
      icon: "lucide:x",
      act: () => win.close(),
      close: true,
    },
  ]
</script>

<div class="flex">
  {#each buttons as button (button.icon)}
    <button
      type="button"
      aria-label={button.label}
      onclick={button.act}
      class={[
        "grid w-12 cursor-pointer place-items-center text-base-content/70",
        "transition-colors",
        button.close
          ? "hover:bg-error hover:text-error-content"
          : "hover:bg-base-content/10 hover:text-base-content",
      ]}
    >
      <Icon
        icon={button.icon}
        class={button.icon === "lucide:square" ? "size-3" : "size-4"}
      />
    </button>
  {/each}
</div>
