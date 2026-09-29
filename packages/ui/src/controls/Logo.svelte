<script lang="ts">
  import { CUBE, DOTS, GLYPH, VIEW } from "./mark"

  type Props = { class?: string; plain?: boolean }

  let { class: klass = "size-6", plain = false }: Props = $props()
</script>

<svg
  viewBox={VIEW}
  role="img"
  aria-label="GPQL"
  class={klass}
  fill="none"
  stroke="currentColor"
  stroke-linecap="round"
  stroke-linejoin="round"
>
  {#if !plain}
    <g stroke-width="9" opacity="0.28">
      {#each CUBE as line (line)}
        <path d={line} />
      {/each}
    </g>

    <g fill="currentColor" stroke="none" opacity="0.34">
      {#each DOTS as [x, y] (`${x}:${y}`)}
        <circle cx={x} cy={y} r="18" />
      {/each}
    </g>

    <g stroke-width="64" opacity="0.22" transform="translate(14,14)">
      {#each GLYPH as line (line)}
        <path d={line} />
      {/each}
    </g>

    <g stroke-width="64" opacity="0.38" transform="translate(7,7)">
      {#each GLYPH as line (line)}
        <path d={line} />
      {/each}
    </g>
  {/if}

  <g stroke-width="64">
    {#each GLYPH as line (line)}
      <path d={line} />
    {/each}
  </g>

  {#if !plain}
    <g stroke-width="10" opacity="0.45" transform="translate(-19,-19)">
      {#each GLYPH as line (line)}
        <path d={line} />
      {/each}
    </g>
  {/if}
</svg>
