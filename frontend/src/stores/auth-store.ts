import { create } from 'zustand'
import { persist } from 'zustand/middleware'

type UserRole = 'admin' | 'user'

interface AuthUser {
  username: string
  role: UserRole
}

interface AuthState {
  token: string | null
  user: AuthUser | null
  login: (token: string, user: AuthUser) => void
  logout: () => void
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: null,
      user: null,
      login: (token, user) => set({ token, user }),
      logout: () => set({ token: null, user: null }),
    }),
    { name: 'modelgate-auth' },
  ),
)
