<script lang="ts">
  import "./layout.css"

  import { MenuHost } from "@gpql/ui"

  let { children } = $props()
</script>

{@render children()}

<MenuHost />
