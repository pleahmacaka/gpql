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

<figure class="overflow-hidden rounded-box bg-base-200 lift">
  <WindowChrome {chip} {chipIcon} {tab} />

  {@render children()}
</figure>
