<script lang="ts">
  import { authClient } from "$lib/auth-client"
  import { Icon, Logo } from "@gpql/ui"

  import type { PageData } from "./$types"

  let { data }: { data: PageData } = $props()

  const start = (provider: "github") =>
    authClient.signIn.social({
      provider,
      callbackURL: `/account${data.handoff}`,
    })
</script>

<svelte:head>
  <title>GPQL account</title>
</svelte:head>

<div class="mx-auto max-w-lg px-6 py-16">
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
        class="flex w-full items-center justify-center gap-2 rounded-field
          bg-neutral py-2.5 text-sm text-neutral-content hover:bg-neutral/90"
      >
        <Icon icon="lucide:github" class="size-4" />
        Continue with GitHub
      </button>
    </div>

    <p class="pt-6 text-xs text-base-content/45">
      Sync is free. The app works without an account, forever.
    </p>
  {:else}
    <h1 class="pt-8 font-display text-3xl font-bold tracking-tight">
      {data.account.name}
    </h1>

    <p class="pt-1 text-sm text-base-content/55">{data.account.email}</p>

    <section class="mt-8 rounded-box bg-base-100 p-5 lift">
      <p class="flex items-center gap-2 text-sm">
        <Icon icon="lucide:check" class="size-4 text-primary" />
        Signed in. GPQL picks it up on this machine.
      </p>
    </section>

    <button
      type="button"
      onclick={() => authClient.signOut()}
      class="mt-6 text-sm text-error hover:underline"
    >
      Sign out
    </button>
  {/if}
</div>
