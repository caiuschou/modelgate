import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { useMyTeams } from '@/features/teams/hooks/use-teams'
import { useAuthStore } from '@/stores/auth-store'
import { useTeamStore } from '@/stores/team-store'
import { useUiStore } from '@/stores/ui-store'

const menuItems = [
  { to: '/', label: '首页' },
  { to: '/api-keys', label: 'API 密钥' },
  { to: '/byok-profiles', label: 'BYOK' },
  { to: '/teams', label: '团队' },
  { to: '/users', label: '用户管理' },
  { to: '/logs', label: '日志中心' },
  { to: '/analytics', label: '统计分析' },
  { to: '/settings', label: '系统设置' },
]

export function AppLayout() {
  const location = useLocation()
  const navigate = useNavigate()
  const user = useAuthStore((state) => state.user)
  const logout = useAuthStore((state) => state.logout)
  const currentTeamId = useTeamStore((s) => s.currentTeamId)
  const setTeamContext = useTeamStore((s) => s.setTeamContext)
  const { data: teamsRes } = useMyTeams()
  const sidebarCollapsed = useUiStore((state) => state.sidebarCollapsed)
  const toggleSidebar = useUiStore((state) => state.toggleSidebar)
  const theme = useUiStore((state) => state.theme)
  const setTheme = useUiStore((state) => state.setTheme)

  const handleLogout = () => {
    logout()
    navigate('/login')
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="sticky top-0 z-40 flex h-14 items-center justify-between border-b border-border bg-card px-4">
        <div className="flex items-center gap-3">
          <Button variant="outline" size="sm" onClick={toggleSidebar}>
            {sidebarCollapsed ? '展开' : '折叠'}
          </Button>
          <span className="font-semibold">ModelGate Console</span>
        </div>
        <div className="flex items-center gap-3 text-sm">
          <label className="flex items-center gap-2 text-muted-foreground">
            <span className="hidden sm:inline">空间</span>
            <select
              aria-label="工作空间"
              value={currentTeamId == null ? '' : String(currentTeamId)}
              onChange={(e) => {
                const v = e.target.value
                setTeamContext(v === '' ? null : Number.parseInt(v, 10))
              }}
              className="max-w-[200px] rounded border border-border bg-background px-2 py-1 text-sm text-foreground"
            >
              <option value="">个人空间</option>
              {(teamsRes?.data ?? []).map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </label>
          <select
            value={theme}
            onChange={(event) =>
              setTheme(event.target.value as 'light' | 'dark' | 'system')
            }
            className="rounded border border-border bg-background px-2 py-1 text-sm"
          >
            <option value="system">系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
          <span className="text-muted-foreground">{user?.username ?? 'guest'}</span>
          <Button variant="outline" size="sm" onClick={handleLogout}>
            退出
          </Button>
        </div>
      </header>

      <div className="mx-auto flex w-full max-w-[1400px]">
        <aside
          className={`border-r border-border bg-card p-3 ${sidebarCollapsed ? 'w-20' : 'w-56'}`}
        >
          <nav className="space-y-1">
            {menuItems.map((item) => {
              const isActive =
                item.to === '/'
                  ? location.pathname === '/'
                  : location.pathname === item.to ||
                    location.pathname.startsWith(`${item.to}/`)
              return (
                <Link
                  key={item.to}
                  to={item.to}
                  className={`block rounded px-3 py-2 text-sm ${
                    isActive
                      ? 'bg-primary text-primary-foreground'
                      : 'text-muted-foreground hover:bg-accent hover:text-foreground'
                  }`}
                >
                  {sidebarCollapsed ? item.label.slice(0, 2) : item.label}
                </Link>
              )
            })}
          </nav>
        </aside>

        <main className="min-h-[calc(100vh-56px)] flex-1 p-6">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
