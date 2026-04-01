import type { ReactNode } from 'react'
import { Navigate, createBrowserRouter } from 'react-router-dom'
import { EmptyState } from '@/components/shared/empty-state'
import { AppLayout } from '@/components/layout/app-layout'
import { DashboardPage } from '@/features/dashboard/pages/dashboard-page'
import { LoginPage } from '@/features/auth/pages/login-page'
import { NotFoundPage } from '@/features/common/pages/not-found-page'
import { useAuthStore } from '@/stores/auth-store'

function AuthGuard({ children }: { children: ReactNode }) {
  const token = useAuthStore((state) => state.token)
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
  {
    path: '/',
    element: (
      <AuthGuard>
        <AppLayout />
      </AuthGuard>
    ),
    children: [
      { index: true, element: <DashboardPage /> },
      {
        path: 'channels',
        element: (
          <AdminGuard>
            <PlaceholderPage title="渠道管理" />
          </AdminGuard>
        ),
      },
      { path: 'tokens', element: <PlaceholderPage title="令牌管理" /> },
      {
        path: 'users',
        element: (
          <AdminGuard>
            <PlaceholderPage title="用户管理" />
          </AdminGuard>
        ),
      },
      { path: 'logs', element: <PlaceholderPage title="日志中心" /> },
      { path: 'analytics', element: <PlaceholderPage title="统计分析" /> },
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
