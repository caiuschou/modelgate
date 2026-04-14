import { create } from 'zustand'

interface AuditLogWsState {
  connected: boolean
  setConnected: (v: boolean) => void
}

/** True when the audit WebSocket is open (console can rely on push instead of polling). */
export const useAuditLogWsStore = create<AuditLogWsState>((set) => ({
  connected: false,
  setConnected: (connected) => set({ connected }),
}))
