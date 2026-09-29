import type { LayoutChannel } from "@gpql/ui"
import * as Y from "yjs"

import type { LayoutState } from "$lib/types"

const BATCH_MS = 200
const POLL_MS = 1000

function encode(bytes: Uint8Array) {
  let text = ""

  for (const byte of bytes) {
    text += String.fromCharCode(byte)
  }

  return btoa(text)
}

function decode(text: string) {
  return Uint8Array.from(atob(text), c => c.charCodeAt(0))
}

async function refusal(answer: Response) {
  const body: unknown = await answer.json().catch(() => null)

  if (
    body &&
    typeof body === "object" &&
    "message" in body &&
    typeof body.message === "string"
  ) {
    return body.message
  }

  return `${answer.status} ${answer.statusText}`
}

export function layoutChannel(
  room: string,
  refused: (message: string) => void,
): LayoutChannel {
  const path = `/api/erd/${room}/layout`

  let pending: Uint8Array[] = []
  let queued = false
  let epoch: number | undefined

  const later = (ms: number) => {
    if (!queued) {
      queued = true
      setTimeout(flush, ms)
    }
  }

  const flush = async () => {
    queued = false

    if (pending.length === 0) {
      return
    }

    const update = Y.mergeUpdates(pending)
    const built = epoch

    pending = []

    const answer = await fetch(path, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ update: encode(update), epoch: built }),
    }).catch(() => null)

    if (!answer || answer.status >= 500) {
      if (built === epoch) {
        pending.unshift(update)
      }

      later(POLL_MS)

      return
    }

    if (!answer.ok) {
      refused(await refusal(answer))
    }
  }

  return {
    send: update => {
      pending.push(update)
      later(BATCH_MS)
    },

    listen: receive => {
      let version = -1
      let busy = false
      let stopped = false

      const poll = async () => {
        if (stopped || busy || document.visibilityState !== "visible") {
          return
        }

        busy = true

        const known = epoch === undefined ? "" : `&epoch=${epoch}`
        const answer = await fetch(`${path}?version=${version}${known}`, {
          cache: "no-store",
        }).catch(() => null)

        if (answer && !answer.ok && answer.status < 500) {
          refused(await refusal(answer))
        }

        const layout: LayoutState | null = answer?.ok
          ? await answer.json().catch(() => null)
          : null

        busy = false

        if (!layout || stopped) {
          return
        }

        // updates queued from the doc a new epoch replaces would carry its history back
        if (epoch !== undefined && layout.epoch !== epoch) {
          pending = []
        }

        epoch = layout.epoch

        if (layout.state) {
          receive(decode(layout.state), layout.epoch)
        }

        version = layout.version
      }

      const ticker = setInterval(poll, POLL_MS)

      document.addEventListener("visibilitychange", poll)
      poll()

      return () => {
        stopped = true
        clearInterval(ticker)
        document.removeEventListener("visibilitychange", poll)
      }
    },
  }
}
