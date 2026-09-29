import * as Y from "yjs"

import type { LayoutChannel } from "../types"
import type { Spot } from "./board.svelte"

type Moved = (spots: Record<string, Spot>, whole: boolean) => void

export function shareLayout(line: LayoutChannel, moved: Moved) {
  let doc = new Y.Doc()
  let positions = doc.getMap<Spot>("positions")
  let epoch: number | undefined

  const watch = () => {
    positions.observe(event => {
      const changed: Record<string, Spot> = {}

      for (const key of event.keysChanged) {
        const spot = positions.get(key)

        if (spot) {
          changed[key] = spot
        }
      }

      moved(changed, false)
    })

    doc.on("update", (update: Uint8Array, origin: unknown) => {
      if (origin !== "remote") {
        line.send(update)
      }
    })
  }

  // merging a compacted state into the old doc would restore its whole history
  const restart = (state: Uint8Array) => {
    doc.destroy()
    doc = new Y.Doc()
    positions = doc.getMap<Spot>("positions")
    Y.applyUpdate(doc, state, "remote")
    watch()
    moved(Object.fromEntries(positions.entries()), true)
  }

  watch()

  const stop = line.listen((update, next) => {
    if (next !== undefined && next !== epoch) {
      epoch = next
      restart(update)

      return
    }

    Y.applyUpdate(doc, update, "remote")
  })

  line.send(Y.encodeStateAsUpdate(doc))

  return {
    keep(spots: Record<string, Spot>) {
      doc.transact(() => {
        for (const [id, spot] of Object.entries(spots)) {
          const held = positions.get(id)

          if (held?.x !== spot.x || held?.y !== spot.y) {
            positions.set(id, spot)
          }
        }
      })
    },

    stop() {
      stop()
      doc.destroy()
    },
  }
}
