import { describe, expect, it } from 'vitest'
import { useAuthStore } from '@/stores/auth-store'

describe('auth store', () => {
  it('supports login and logout', () => {
    useAuthStore.setState({ token: null, user: null })

    useAuthStore.getState().login('token-1', {
      username: 'admin',
      role: 'admin',
    })

    expect(useAuthStore.getState().token).toBe('token-1')
    expect(useAuthStore.getState().user?.username).toBe('admin')

    useAuthStore.getState().logout()

    expect(useAuthStore.getState().token).toBeNull()
    expect(useAuthStore.getState().user).toBeNull()
  })
})
