export interface ErdRoomSummary {
  id: string
  name: string
  open: boolean
  createdAt: number
}

export interface LayoutState {
  version: number
  epoch: number
  state: string | null
}
