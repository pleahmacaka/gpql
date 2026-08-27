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
  optimizeDeps: {
    exclude: ["@gpql/ui"],
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
