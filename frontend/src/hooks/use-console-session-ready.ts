import { useEffect, useState } from 'react'
import { useAuthStore } from '@/stores/auth-store'

/**
 * After Zustand auth persist rehydrates, console JWT is available for `apiClient`.
 * Query `enabled` should use this so the first fetch never runs without `Authorization`.
 */
export function useConsoleSessionReady(): boolean {
  const token = useAuthStore((s) => s.token)
  const [authHydrated, setAuthHydrated] = useState(() =>
    useAuthStore.persist.hasHydrated(),
  )

  useEffect(() => {
    const finish = () => {
      setAuthHydrated(true)
    }
    if (useAuthStore.persist.hasHydrated()) {
      queueMicrotask(finish)
      return
    }
    return useAuthStore.persist.onFinishHydration(finish)
  }, [])

  return authHydrated && Boolean(token)
}
