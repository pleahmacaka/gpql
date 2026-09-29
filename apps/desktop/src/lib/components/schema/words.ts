import type { Words } from "@gpql/ui"

import * as m from "$lib/paraglide/messages"

export const boardWords = (): Partial<Words> => ({
  auto: m.arrange_auto(),
  picked: m.arrange_picked(),
  group: m.group_make(),
  ungroup: m.group_drop(),
  rename: m.group_rename(),
  warn: m.arrange_warn(),
  groupName: m.group_name(),
  think: m.group_ai(),
  nothing: m.group_ai_none(),
  rest: m.group_rest(),
  define: m.menu_ddl(),
  cancel: m.cancel(),
  dismiss: m.close(),
  open: m.board_open_rows(),
  referenced: m.board_referenced(),
  level: m.board_level(),
  primary: m.board_primary(),
  nullable: m.board_nullable(),
  references: m.board_references(),
  board: m.board_label(),
  keys: m.board_keys(),
  controls: m.board_zoom(),
  zoomIn: m.board_zoom_in(),
  zoomOut: m.board_zoom_out(),
  fit: m.board_fit(),
  minimap: m.option_minimap(),
})
