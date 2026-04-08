import { useEffect, useState } from 'react'
import { Navigate } from 'react-router-dom'
import { FeaturesHomePage } from '@/features/home/pages/features-home-page'
import { useAuthStore } from '@/stores/auth-store'

/**
 * 已登录用户不展示营销特性页，直接进入控制台仪表盘。
 */
export function FeaturesHomeGate() {
  const token = useAuthStore((s) => s.token)
  const [hydrated, setHydrated] = useState(() =>
    useAuthStore.persist.hasHydrated(),
  )

  useEffect(() => {
    const done = () => setHydrated(true)
    if (useAuthStore.persist.hasHydrated()) {
      queueMicrotask(done)
      return
    }
    return useAuthStore.persist.onFinishHydration(done)
  }, [])

  if (!hydrated) {
    return (
      <div className="flex min-h-[40vh] items-center justify-center text-sm text-muted-foreground">
        加载中…
      </div>
    )
  }

  if (token) {
    return <Navigate to="/dashboard" replace />
  }

  return <FeaturesHomePage />
}
