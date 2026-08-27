import { resolve } from "node:path"

import adapter from "@sveltejs/adapter-vercel"
import { sveltekit } from "@sveltejs/kit/vite"
import tailwindcss from "@tailwindcss/vite"
import type { Plugin } from "vite"
import { defineConfig } from "vite"
import { attachErdSocket } from "./src/erd-socket.ts"

function erdSocket(): Plugin {
  return {
    name: "gpql-erd-socket",
    configureServer(server) {
      if (server.httpServer) {
        attachErdSocket(server.httpServer)
      }
    },
  }
}

export default defineConfig({
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
  plugins: [
    erdSocket(),
    tailwindcss(),
    sveltekit({
      compilerOptions: {
        // Force runes mode for the project, except for libraries. Can be removed in svelte 6.
        runes: ({ filename }) =>
          filename.split(/[/\\]/).includes("node_modules") ? undefined : true,
      },
      adapter: adapter({ runtime: "nodejs22.x" }),
      typescript: {
        config: config => {
          config.include.push("../drizzle.config.ts")
        },
      },
    }),
  ],
})
