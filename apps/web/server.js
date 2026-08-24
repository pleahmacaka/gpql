import { createServer } from "node:http"

import { handler } from "./build/handler.js"

import { attachErdSocket } from "./build/erd-socket.js"

const port = Number(process.env.PORT ?? 3000)
const server = createServer(handler)

attachErdSocket(server)
server.listen(port)
