import { addCollection } from "@iconify/svelte"

import lucide from "./packs/lucide.json"
import simple from "./packs/simple-icons.json"

addCollection(lucide)
addCollection(simple)

export { default as Icon } from "@iconify/svelte"
