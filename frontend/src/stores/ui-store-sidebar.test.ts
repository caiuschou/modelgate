import { beforeEach, describe, expect, it } from 'vitest'
import { useUiStore } from '@/stores/ui-store'

describe('ui store sidebar', () => {
  beforeEach(() => {
    localStorage.removeItem('modelgate-ui')
    useUiStore.setState({ sidebarCollapsed: false, theme: 'system' })
  })

  it('setSidebarExpanded true means expanded (not collapsed)', () => {
    useUiStore.setState({ sidebarCollapsed: true })
    useUiStore.getState().setSidebarExpanded(true)
    expect(useUiStore.getState().sidebarCollapsed).toBe(false)
  })

  it('setSidebarExpanded false means collapsed', () => {
    useUiStore.setState({ sidebarCollapsed: false })
    useUiStore.getState().setSidebarExpanded(false)
    expect(useUiStore.getState().sidebarCollapsed).toBe(true)
  })
})
