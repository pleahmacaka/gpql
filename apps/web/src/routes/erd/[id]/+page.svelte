<script lang="ts">
  import { Icon, SchemaBoard } from "@gpql/ui"
  import { SvelteFlowProvider } from "@xyflow/svelte"

  import type { PageData } from "./$types"

  let { data }: { data: PageData } = $props()

  let room = $derived(
    typeof window === "undefined"
      ? ""
      : `${window.location.protocol === "https:" ? "wss" : "ws"}://${window.location.host}/erd-socket/${data.room.id}`,
  )
</script>

<svelte:head>
  <title>{data.room.name}, GPQL</title>
</svelte:head>

<div class="flex h-screen flex-col">
  <header class="flex items-center gap-3 px-4 py-3 sm:px-6">
    <a href="/" class="font-display text-base font-medium">GPQL</a>

    <span class="text-sm text-base-content/45">{data.room.name}</span>

    <span class="flex-1"></span>

    <span class="flex items-center gap-2 text-xs text-base-content/45">
      <Icon icon="lucide:users" class="size-3.5" />
      {data.who}
    </span>
  </header>

  <div class="min-h-0 flex-1 px-4 pb-4 sm:px-6 sm:pb-6">
    <div class="h-full overflow-hidden rounded-box bg-base-100 lift">
      <SvelteFlowProvider>
        <SchemaBoard tables={data.room.tables} {room} />
      </SvelteFlowProvider>
    </div>
  </div>
</div>
