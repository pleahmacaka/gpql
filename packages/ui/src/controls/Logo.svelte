<script lang="ts">
  type Props = { class?: string; plain?: boolean }

  let { class: klass = "size-6", plain = false }: Props = $props()

  const GLYPH = [
    "M950 437 L630 252 L310 437 L310 807 L630 992 L800 894",
    "M630 622 L800 720 L800 992",
    "M950 769 L800 856 L950 943",
  ]

  const CUBE = [
    "M630 426 L800 524 L800 720 L630 818 L460 720 L460 524 Z",
    "M630 622 L800 524",
    "M630 622 L460 524",
    "M630 622 L630 818",
  ]

  const DOTS = [
    [630, 426],
    [800, 524],
    [800, 720],
    [630, 818],
    [460, 720],
    [460, 524],
  ]
</script>

<svg
  viewBox="188 180 884 884"
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
