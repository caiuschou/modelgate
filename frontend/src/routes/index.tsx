import type { ReactNode } from 'react'
import { useEffect, useState } from 'react'
import { Navigate, Outlet, createBrowserRouter } from 'react-router-dom'
import { EmptyState } from '@/components/shared/empty-state'
import { AppLayout } from '@/components/layout/app-layout'
import { DocumentTitleSync } from '@/lib/document-title'
import { DashboardPage } from '@/features/dashboard/pages/dashboard-page'
import { LoginPage } from '@/features/auth/pages/login-page'
import { RegisterPage } from '@/features/auth/pages/register-page'
import { NotFoundPage } from '@/features/common/pages/not-found-page'
import { ApiKeysPage } from '@/features/api-keys/pages/api-keys-page'
import { ApiKeyDetailPage } from '@/features/api-keys/pages/api-key-detail-page'
import { ByokProfilesPage } from '@/features/byok/pages/byok-profiles-page'
import { ByokProfileDetailPage } from '@/features/byok/pages/byok-profile-detail-page'
import { LogDetailPage } from '@/features/logs/pages/log-detail-page'
import { LogListPage } from '@/features/logs/pages/log-list-page'
import { LogListV2Page } from '@/features/logs/pages/log-list-v2-page'
import { AnalyticsPage } from '@/features/analytics/pages/analytics-page'
import { TeamsPage } from '@/features/teams/pages/teams-page'
import { TeamMembersPage } from '@/features/teams/pages/team-members-page'
import { AcceptInvitePage } from '@/features/teams/pages/accept-invite-page'
import { FeaturesHomeGate } from '@/features/home/pages/features-home-gate'
import { ModelsCatalogPage } from '@/features/models/pages/models-catalog-page'
import { AdminRechargePage } from '@/features/billing/pages/admin-recharge-page'
import { BillingPage } from '@/features/billing/pages/billing-page'
import { useAuthStore } from '@/stores/auth-store'

const DEFAULT_ADMIN_RECHARGE_SEGMENT = '__mg-admin-recharge'

/**
 * Hidden admin top-up URL segment (no leading slash). Set `VITE_ADMIN_RECHARGE_PATH` for production
 * (must match deployment URL). In dev, defaults if env is missing so the page works without `.env`.
 */
function adminRechargeRouteSegment(): string {
  const raw = import.meta.env.VITE_ADMIN_RECHARGE_PATH
  if (typeof raw === 'string' && raw.trim() !== '') {
    return raw.trim().replace(/^\/+/, '')
  }
  if (import.meta.env.DEV) {
    return DEFAULT_ADMIN_RECHARGE_SEGMENT
  }
  return ''
}

const ADMIN_RECHARGE_SEGMENT = adminRechargeRouteSegment()

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
    return <Navigate to="/dashboard" replace />
  }
  return <>{children}</>
}

function PlaceholderPage({ title }: { title: string }) {
  return (
    <EmptyState title={title} description="页面建设中。" />
  )
}

function AuthGuardOutlet() {
  return (
    <AuthGuard>
      <Outlet />
    </AuthGuard>
  )
}

function RootLayout() {
  return (
    <>
      <DocumentTitleSync />
      <Outlet />
    </>
  )
}

export const router = createBrowserRouter([
  {
    element: <RootLayout />,
    children: [
      { path: '/login', element: <LoginPage />, handle: { pageTitle: '登录' } },
      { path: '/register', element: <RegisterPage />, handle: { pageTitle: '注册' } },
      ...(ADMIN_RECHARGE_SEGMENT
        ? [
            {
              path: `/${ADMIN_RECHARGE_SEGMENT}`,
              element: <AdminRechargePage />,
              handle: { pageTitle: '管理充值' },
            },
          ]
        : []),
      {
        path: '/',
        element: <AppLayout />,
        children: [
          { index: true, element: <FeaturesHomeGate />, handle: { pageTitle: '首页' } },
          { path: 'models', element: <ModelsCatalogPage />, handle: { pageTitle: 'Models' } },
          {
            element: <AuthGuardOutlet />,
            children: [
              { path: 'dashboard', element: <DashboardPage />, handle: { pageTitle: '控制台' } },
              { path: 'teams', element: <TeamsPage />, handle: { pageTitle: '团队' } },
              {
                path: 'teams/:teamId/members',
                element: <TeamMembersPage />,
                handle: { pageTitle: '团队成员' },
              },
              { path: 'invite', element: <AcceptInvitePage />, handle: { pageTitle: '邀请' } },
              { path: 'api-keys', element: <ApiKeysPage />, handle: { pageTitle: 'API 密钥' } },
              {
                path: 'api-keys/:id',
                element: <ApiKeyDetailPage />,
                handle: { pageTitle: 'API 密钥详情' },
              },
              {
                path: 'byok-profiles',
                element: <ByokProfilesPage />,
                handle: { pageTitle: 'BYOK' },
              },
              {
                path: 'byok-profiles/:id',
                element: <ByokProfileDetailPage />,
                handle: { pageTitle: 'BYOK 详情' },
              },
              {
                path: 'users',
                element: (
                  <AdminGuard>
                    <PlaceholderPage title="用户管理" />
                  </AdminGuard>
                ),
                handle: { pageTitle: '用户管理' },
              },
              { path: 'logs', element: <LogListPage />, handle: { pageTitle: '日志' } },
              {
                path: 'logs/v2',
                element: <LogListV2Page />,
                handle: { pageTitle: '会话中心' },
              },
              {
                path: 'logs/:requestId',
                element: <LogDetailPage />,
                handle: { pageTitle: '日志详情' },
              },
              { path: 'analytics', element: <AnalyticsPage />, handle: { pageTitle: '统计分析' } },
              { path: 'billing', element: <BillingPage />, handle: { pageTitle: '充值中心' } },
              { path: 'account/password', element: <Navigate to="/dashboard" replace /> },
              {
                path: 'settings',
                element: (
                  <AdminGuard>
                    <PlaceholderPage title="系统设置" />
                  </AdminGuard>
                ),
                handle: { pageTitle: '系统设置' },
              },
              { path: '*', element: <NotFoundPage />, handle: { pageTitle: '页面未找到' } },
            ],
          },
        ],
      },
      { path: '*', element: <NotFoundPage />, handle: { pageTitle: '页面未找到' } },
    ],
  },
])
