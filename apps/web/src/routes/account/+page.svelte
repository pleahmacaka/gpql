<script lang="ts">
  import { enhance } from "$app/forms"
  import { invalidateAll } from "$app/navigation"
  import { authClient } from "$lib/auth-client"
  import { Icon, Logo } from "@gpql/ui"

  import type { ActionData, PageData } from "./$types"

  let { data, form }: { data: PageData; form: ActionData } = $props()

  let starting = $state(false)
  let connecting = $state(false)

  let back = $derived(
    data.next ??
      (data.handoff
        ? `/account?port=${data.handoff.port}&state=${data.handoff.state}` +
          (data.post ? "&form=1" : "")
        : "/account"),
  )

  const start = async (provider: "github") => {
    if (starting) {
      return
    }

    starting = true

    try {
      await authClient.signIn.social({ provider, callbackURL: back })
    } catch {
      starting = false
    }
  }

  const leave = async () => {
    await authClient.signOut()
    await invalidateAll()
  }
</script>

<svelte:head>
  <title>GPQL account</title>
</svelte:head>

<div class="mx-auto max-w-lg px-4 py-12 sm:px-6 sm:py-16">
  <a href="/" class="flex items-center gap-2 font-display text-base font-medium">
    <Logo class="size-5" />
    GPQL
  </a>

  {#if !data.account}
    <h1 class="pt-8 font-display text-3xl font-bold tracking-tight">
      Sign in to sync
    </h1>

    <p class="pt-3 text-base-content/65">
      Settings, connections and saved queries travel with the account. Passwords never do.
    </p>

    <div class="space-y-2 pt-7">
      <button
        type="button"
        onclick={() => start("github")}
        disabled={starting}
        class="cursor-pointer disabled:cursor-not-allowed flex w-full items-center justify-center gap-2 rounded-field
          bg-neutral py-3 text-sm text-neutral-content hover:bg-neutral/90
          disabled:opacity-60"
      >
        <Icon
          icon={starting ? "lucide:loader-circle" : "lucide:github"}
          class="size-4 {starting ? 'animate-spin' : ''}"
        />
        {starting ? "Opening GitHub" : "Continue with GitHub"}
      </button>
    </div>

    <p class="pt-6 text-xs text-base-content/45">
      Sync is free. The app works without an account, forever.
    </p>
  {:else if data.handoff}
    <h1 class="pt-8 font-display text-3xl font-bold tracking-tight">
      Connect GPQL desktop
    </h1>

    <p class="pt-3 text-base-content/65">
      GPQL on this computer asked to sync as
      <span class="text-base-content">{data.account.name}</span>
      ({data.account.email}). It is waiting on port {data.handoff.port}.
    </p>

    <p class="pt-3 text-sm text-base-content/55">
      Connect only if you just chose Sign in inside GPQL.
    </p>

    {#if form?.desktop}
      <form
        method={form.desktop.post ? "POST" : "GET"}
        action="http://127.0.0.1:{form.desktop.port}/"
        {@attach sender => sender.submit()}
      >
        <input type="hidden" name="token" value={form.desktop.token} />
        <input type="hidden" name="state" value={form.desktop.state} />
      </form>

      <p class="flex items-center gap-2 pt-7 text-sm">
        <Icon icon="lucide:loader-circle" class="size-4 animate-spin" />
        Handing over to GPQL desktop
      </p>
    {:else}
      <form
        method="POST"
        action="?/connect"
        class="flex items-center gap-4 pt-7"
        use:enhance={() => {
          connecting = true

          return async ({ update }) => {
            await update({ reset: false })
            connecting = false
          }
        }}
      >
        <input type="hidden" name="port" value={data.handoff.port} />
        <input type="hidden" name="state" value={data.handoff.state} />
        {#if data.post}
          <input type="hidden" name="form" value="1" />
        {/if}

        <button
          type="submit"
          disabled={connecting}
          class="cursor-pointer disabled:cursor-not-allowed flex items-center gap-2 rounded-field bg-primary px-4 py-3 text-sm
            text-primary-content hover:bg-primary/90 disabled:opacity-60"
        >
          <Icon
            icon={connecting ? "lucide:loader-circle" : "lucide:monitor"}
            class="size-4 {connecting ? 'animate-spin' : ''}"
          />
          Connect
        </button>

        <a href="/account" class="text-sm text-base-content/65 hover:underline">
          Not now
        </a>
      </form>

      {#if form?.message}
        <p class="pt-4 text-sm text-error">{form.message}</p>
      {/if}
    {/if}
  {:else}
    <h1 class="pt-8 font-display text-3xl font-bold tracking-tight">
      {data.account.name}
    </h1>

    <p class="pt-1 text-sm text-base-content/55">{data.account.email}</p>

    <section class="mt-8 rounded-box bg-base-100 p-5 lift">
      <p class="flex items-center gap-2 text-sm">
        <Icon icon="lucide:check" class="size-4 text-primary" />
        Signed in. Choose Sign in inside GPQL to sync this computer.
      </p>
    </section>

    <button
      type="button"
      onclick={leave}
      class="cursor-pointer mt-6 text-sm text-error hover:underline"
    >
      Sign out
    </button>
  {/if}
</div>
