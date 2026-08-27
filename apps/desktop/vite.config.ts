import { resolve } from "node:path"

import { compile, paraglideVitePlugin } from "@inlang/paraglide-js"
import adapter from "@sveltejs/adapter-static"
import { sveltekit } from "@sveltejs/kit/vite"
import tailwindcss from "@tailwindcss/vite"

import { defineConfig } from "vite"

import { version } from "./package.json"

// the paraglide plugin only watches its project file, so message edits need a
// nudge of their own
function messages() {
  let busy = false
  let soon: ReturnType<typeof setTimeout> | undefined

  return {
    name: "gpql-messages",
    configureServer(server: {
      watcher: {
        add: (path: string) => void
        on: (event: string, run: (file: string) => void) => void
      }
      hot: { send: (payload: { type: string }) => void }
    }) {
      server.watcher.add(resolve("messages"))

      server.watcher.on("change", (file: string) => {
        const path = file.replaceAll("\\", "/")

        if (busy || !path.includes("/messages/") || !path.endsWith(".json")) {
          return
        }

        clearTimeout(soon)

        soon = setTimeout(async () => {
          busy = true

          try {
            await compile({
              project: "./project.inlang",
              outdir: "./src/lib/paraglide",
              cleanOutdir: false,
            })
            server.hot.send({ type: "full-reload" })
          } finally {
            busy = false
          }
        }, 200)
      })
    },
  }
}

export default defineConfig({
  clearScreen: false,
  resolve: {
    // the workspace symlink lands in node_modules, which vite neither
    // watches nor HMRs; point straight at the source instead
    alias: [
      {
        find: /^@gpql\/ui$/,
        replacement: resolve(
          import.meta.dirname,
          "../../packages/ui/src/index.ts",
        ),
      },
      {
        find: /^@gpql\/ui\//,
        replacement: `${resolve(import.meta.dirname, "../../packages/ui/src")}/`,
      },
    ],
  },
  define: {
    __GPQL_VERSION__: JSON.stringify(version),
  },
  server: {
    port: 1421,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
    fs: { allow: ["./drizzle"] },
  },
  plugins: [
    messages(),
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./src/lib/paraglide",
      strategy: ["localStorage", "preferredLanguage", "baseLocale"],
    }),
    tailwindcss(),
    sveltekit({
      compilerOptions: {
        // Force runes mode for the project, except for libraries. Can be removed in svelte 6.
        runes: ({ filename }) =>
          filename.split(/[/\\]/).includes("node_modules") ? undefined : true,
      },
      adapter: adapter(),
    }),
  ],
})
