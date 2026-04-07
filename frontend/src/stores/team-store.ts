import { create } from 'zustand'
import { persist } from 'zustand/middleware'

/** `null` = personal (non-team) console context for API keys / logs / analytics. */
interface TeamState {
  currentTeamId: number | null
  setTeamContext: (teamId: number | null) => void
}

export const useTeamStore = create<TeamState>()(
  persist(
    (set) => ({
      currentTeamId: null,
      setTeamContext: (teamId) => set({ currentTeamId: teamId }),
    }),
    { name: 'modelgate-team' },
  ),
)
