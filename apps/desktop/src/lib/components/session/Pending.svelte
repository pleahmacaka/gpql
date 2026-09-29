<script lang="ts">
  type Props = { label: string; rows?: number }

  let { label, rows = 3 }: Props = $props()

  const WIDTHS = ["w-2/5", "w-1/2", "w-1/3"]
</script>

<div role="status" class="flex max-h-full flex-col overflow-hidden">
  <span class="sr-only">{label}</span>

  {#each { length: rows }, index}
    <div aria-hidden="true" class="flex h-13 items-center gap-3 px-4">
      <span class="size-4 shrink-0 animate-pulse bg-base-content/10"></span>

      <span class="flex min-w-0 flex-1 flex-col gap-2">
        <span
          class={["h-3 animate-pulse bg-base-content/10", WIDTHS[index % 3]]}
        ></span>
        <span class="h-2 w-1/4 animate-pulse bg-base-content/10"></span>
      </span>
    </div>
  {/each}
</div>
