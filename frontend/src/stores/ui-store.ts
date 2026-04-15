import { create } from 'zustand'
import { persist } from 'zustand/middleware'

type Theme = 'light' | 'dark' | 'system'

interface UiState {
  sidebarCollapsed: boolean
  theme: Theme
  /** `true` = 侧栏展开（与 shadcn `SidebarProvider` 的 `open` 一致） */
  setSidebarExpanded: (expanded: boolean) => void
  setTheme: (theme: Theme) => void
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      sidebarCollapsed: false,
      theme: 'system',
      setSidebarExpanded: (expanded) => set({ sidebarCollapsed: !expanded }),
      setTheme: (theme) => set({ theme }),
    }),
    { name: 'modelgate-ui' },
  ),
)
