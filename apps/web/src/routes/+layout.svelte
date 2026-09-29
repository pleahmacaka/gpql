<script lang="ts">
  import "pretendard/dist/web/variable/pretendardvariable-dynamic-subset.css"
  import "./layout.css"

  import MenuHost from "@gpql/ui/controls/MenuHost.svelte"

  let { children } = $props()
</script>

{@render children()}

<MenuHost />
