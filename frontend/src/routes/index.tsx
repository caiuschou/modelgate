import type { ReactNode } from 'react'
import { useEffect, useState } from 'react'
import { Navigate, createBrowserRouter } from 'react-router-dom'
import { EmptyState } from '@/components/shared/empty-state'
import { AppLayout } from '@/components/layout/app-layout'
import { DashboardPage } from '@/features/dashboard/pages/dashboard-page'
import { LoginPage } from '@/features/auth/pages/login-page'
import { RegisterPage } from '@/features/auth/pages/register-page'
import { NotFoundPage } from '@/features/common/pages/not-found-page'
import { ApiKeysPage } from '@/features/api-keys/pages/api-keys-page'
import { ApiKeyDetailPage } from '@/features/api-keys/pages/api-key-detail-page'
import { LogDetailPage } from '@/features/logs/pages/log-detail-page'
import { LogListPage } from '@/features/logs/pages/log-list-page'
import { AnalyticsPage } from '@/features/analytics/pages/analytics-page'
import { TeamsPage } from '@/features/teams/pages/teams-page'
import { TeamMembersPage } from '@/features/teams/pages/team-members-page'
import { AcceptInvitePage } from '@/features/teams/pages/accept-invite-page'
import { useAuthStore } from '@/stores/auth-store'

function AuthGuard({ children }: { children: ReactNode }) {
  const token = useAuthStore((state) => state.token)
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

  if (!authHydrated) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background p-6 text-sm text-muted-foreground">
        加载中…
      </div>
    )
  }
  if (!token) {
    return (
      <Navigate
        to={`/login?redirect=${encodeURIComponent(window.location.pathname)}`}
        replace
      />
    )
  }
  return <>{children}</>
}

function AdminGuard({ children }: { children: ReactNode }) {
  const user = useAuthStore((state) => state.user)
  if (user?.role !== 'admin') {
    return <Navigate to="/" replace />
  }
  return <>{children}</>
}

function PlaceholderPage({ title }: { title: string }) {
  return (
    <EmptyState title={title} description="页面建设中。" />
  )
}

export const router = createBrowserRouter([
  { path: '/login', element: <LoginPage /> },
  { path: '/register', element: <RegisterPage /> },
  {
    path: '/',
    element: (
      <AuthGuard>
        <AppLayout />
      </AuthGuard>
    ),
    children: [
      { index: true, element: <DashboardPage /> },
      { path: 'teams', element: <TeamsPage /> },
      { path: 'teams/:teamId/members', element: <TeamMembersPage /> },
      { path: 'invite', element: <AcceptInvitePage /> },
      { path: 'api-keys', element: <ApiKeysPage /> },
      { path: 'api-keys/:id', element: <ApiKeyDetailPage /> },
      {
        path: 'users',
        element: (
          <AdminGuard>
            <PlaceholderPage title="用户管理" />
          </AdminGuard>
        ),
      },
      { path: 'logs', element: <LogListPage /> },
      { path: 'logs/:requestId', element: <LogDetailPage /> },
      { path: 'analytics', element: <AnalyticsPage /> },
      {
        path: 'settings',
        element: (
          <AdminGuard>
            <PlaceholderPage title="系统设置" />
          </AdminGuard>
        ),
      },
      { path: '*', element: <NotFoundPage /> },
    ],
  },
  { path: '*', element: <NotFoundPage /> },
])
