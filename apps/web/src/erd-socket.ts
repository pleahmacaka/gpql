import type { IncomingMessage } from "node:http"
import type { Duplex } from "node:stream"

import { type WebSocket, WebSocketServer } from "ws"

const PREFIX = "/erd-socket/"

type Upgradable = {
  on: (
    event: "upgrade",
    listener: (request: IncomingMessage, socket: Duplex, head: Buffer) => void,
  ) => void
}

export function attachErdSocket(server: Upgradable) {
  const relay = new WebSocketServer({ noServer: true })
  const rooms = new Map<string, Set<WebSocket>>()

  server.on("upgrade", (request, socket, head) => {
    const path = request.url ?? ""

    if (!path.startsWith(PREFIX)) {
      return
    }

    const id = path.slice(PREFIX.length).split("?")[0]

    relay.handleUpgrade(request, socket, head, peer => {
      const peers = rooms.get(id) ?? new Set()

      peers.add(peer)
      rooms.set(id, peers)

      peer.on("message", data => {
        for (const other of peers) {
          if (other !== peer && other.readyState === other.OPEN) {
            other.send(data)
          }
        }
      })

      peer.on("close", () => {
        peers.delete(peer)

        if (peers.size === 0) {
          rooms.delete(id)
        }
      })
    })
  })
}
