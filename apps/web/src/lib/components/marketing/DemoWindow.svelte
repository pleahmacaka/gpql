<script lang="ts">
  import { WindowChrome } from "@gpql/ui"
  import type { Snippet } from "svelte"

  type Props = {
    chip: string
    chipIcon?: string
    tab?: string | null
    children: Snippet
  }

  let { chip, chipIcon = "lucide:database", tab = null, children }: Props =
    $props()
</script>

<figure class="overflow-x-auto rounded-box bg-base-200 lift">
  <div class="min-w-160">
    <WindowChrome {chip} {chipIcon} {tab} />

    {@render children()}
  </div>
</figure>
