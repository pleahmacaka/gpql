<script lang="ts">
  import { field } from "@gpql/ui/ascii/field.ts"
  import { Icon } from "@gpql/ui/icons/index.ts"

  import { wire } from "./scenes/wire"

  type Stop = { icon: string; name: string; at: string; sealed?: boolean }

  type Route = { icon: string; label: string; stops: Stop[] }

  const HERE: Stop = { icon: "lucide:monitor", name: "This PC", at: "" }

  const ROUTES: Route[] = [
    {
      icon: "simple-icons:docker",
      label: "Local port or Docker container",
      stops: [
        HERE,
        { icon: "simple-icons:postgresql", name: "pg-main", at: ":5432" },
      ],
    },
    {
      icon: "simple-icons:tailscale",
      label: "Peer on your tailnet",
      stops: [
        HERE,
        {
          icon: "simple-icons:postgresql",
          name: "nas.tail4c2e1.ts.net",
          at: ":5432",
        },
      ],
    },
    {
      icon: "lucide:key-round",
      label: "SSH jump host, password or key",
      stops: [
        HERE,
        { icon: "lucide:server", name: "bastion", at: ":22", sealed: true },
        { icon: "simple-icons:mysql", name: "10.0.4.12", at: ":3306" },
      ],
    },
  ]
</script>

<section
  id="reach"
  aria-labelledby="reach-title"
  class="border-t border-base-content/10"
>
  <div class="mx-auto max-w-6xl px-4 py-24 sm:px-6">
    <p class="flex items-center gap-3 text-sm text-primary">
      <span class="size-2 bg-primary"></span>
      Reach
    </p>

    <h2
      id="reach-title"
      class="pt-4 text-4xl font-extrabold tracking-tight sm:text-6xl"
    >
      Wherever it runs.
    </h2>

    <ol class="grid grid-cols-1 gap-6 pt-12">
      {#each ROUTES as route (route.label)}
        <li
          class={[
            "grid grid-cols-1 items-center gap-3",
            "lg:grid-cols-12 lg:gap-8",
          ]}
        >
          <p
            class={[
              "flex items-center gap-2 text-sm text-base-content/70",
              "lg:col-span-3",
            ]}
          >
            <Icon icon={route.icon} class="size-4 shrink-0 text-primary" />
            {route.label}
          </p>

          <div class="flex items-center lg:col-span-9">
            {#each route.stops as stop, index (stop.name)}
              {#if index > 0}
                <canvas
                  aria-hidden="true"
                  class="h-6 min-w-8 flex-1"
                  {@attach field(wire({ sealed: stop.sealed }))}
                ></canvas>
              {/if}

              <span
                class={[
                  "flex min-w-0 shrink items-center gap-2",
                  "bg-base-100 px-3 py-2 text-sm hairline",
                ]}
              >
                <Icon icon={stop.icon} class="size-4 shrink-0" />
                <span
                  class={stop === HERE ? "sr-only sm:not-sr-only" : "truncate"}
                >
                  {stop.name}
                </span>

                {#if stop.at}
                  <span
                    class="hidden text-base-content/70 tabular-nums sm:inline"
                  >
                    {stop.at}
                  </span>
                {/if}

                {#if stop.sealed}
                  <Icon
                    icon="lucide:lock"
                    class="size-4 shrink-0 text-primary"
                  />
                {/if}
              </span>
            {/each}
          </div>
        </li>
      {/each}
    </ol>
  </div>
</section>
