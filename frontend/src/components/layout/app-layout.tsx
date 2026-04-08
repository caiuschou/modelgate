import { ChevronDown, Monitor, Moon, Sun } from 'lucide-react'
import { useState } from 'react'
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import { cn } from '@/lib/utils'
import { ChangePasswordModal } from '@/features/auth/components/change-password-modal'
import { useMyTeams } from '@/features/teams/hooks/use-teams'
import { useAuthStore } from '@/stores/auth-store'
import { useTeamStore } from '@/stores/team-store'
import { useUiStore } from '@/stores/ui-store'

const themeChoices = [
  { value: 'light' as const, label: '浅色', Icon: Sun },
  { value: 'dark' as const, label: '深色', Icon: Moon },
  { value: 'system' as const, label: '跟随系统', Icon: Monitor },
]

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
  const [workspaceOpen, setWorkspaceOpen] = useState(false)
  const [userMenuOpen, setUserMenuOpen] = useState(false)
  const [changePasswordOpen, setChangePasswordOpen] = useState(false)
  const teams = teamsRes?.data ?? []
  const currentSpaceLabel =
    currentTeamId == null
      ? '个人空间'
      : (teams.find((t) => t.id === currentTeamId)?.name ??
        `团队 #${currentTeamId}`)

  const handleLogout = () => {
    setUserMenuOpen(false)
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
          <Popover open={workspaceOpen} onOpenChange={setWorkspaceOpen}>
            <PopoverTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-auto max-w-[220px] gap-1.5 px-2 py-1.5 font-medium sm:max-w-[280px]"
                aria-label="切换工作空间"
                aria-expanded={workspaceOpen}
              >
                <span className="min-w-0 flex-1 truncate text-left text-sm text-foreground">
                  {currentSpaceLabel}
                </span>
                <ChevronDown
                  className="size-4 shrink-0 text-muted-foreground"
                  aria-hidden
                />
              </Button>
            </PopoverTrigger>
            <PopoverContent align="end" className="w-56 p-1" sideOffset={6}>
              <div
                role="listbox"
                aria-label="工作空间"
                className="flex max-h-[min(60vh,320px)] flex-col gap-0.5 overflow-y-auto"
              >
                <button
                  type="button"
                  role="option"
                  aria-selected={currentTeamId == null}
                  className={cn(
                    'w-full truncate rounded-md px-2 py-1.5 text-left text-sm outline-none transition-colors',
                    'hover:bg-accent hover:text-accent-foreground',
                    'focus-visible:ring-2 focus-visible:ring-ring/50',
                    currentTeamId == null &&
                      'bg-accent text-accent-foreground',
                  )}
                  onClick={() => {
                    setTeamContext(null)
                    setWorkspaceOpen(false)
                  }}
                >
                  个人空间
                </button>
                {teams.map((t) => (
                  <button
                    key={t.id}
                    type="button"
                    role="option"
                    aria-selected={currentTeamId === t.id}
                    className={cn(
                      'w-full truncate rounded-md px-2 py-1.5 text-left text-sm outline-none transition-colors',
                      'hover:bg-accent hover:text-accent-foreground',
                      'focus-visible:ring-2 focus-visible:ring-ring/50',
                      currentTeamId === t.id &&
                        'bg-accent text-accent-foreground',
                    )}
                    onClick={() => {
                      setTeamContext(t.id)
                      setWorkspaceOpen(false)
                    }}
                  >
                    {t.name}
                  </button>
                ))}
              </div>
            </PopoverContent>
          </Popover>
          <div
            data-slot="button-group"
            className="flex items-center rounded-lg border border-border bg-muted/40 p-0.5"
            role="radiogroup"
            aria-label="主题"
          >
            {themeChoices.map(({ value, label, Icon }) => (
              <Button
                key={value}
                type="button"
                role="radio"
                aria-checked={theme === value}
                variant={theme === value ? 'secondary' : 'ghost'}
                size="icon-sm"
                className="shrink-0"
                title={label}
                aria-label={label}
                onClick={() => setTheme(value)}
              >
                <Icon className="size-4" aria-hidden />
              </Button>
            ))}
          </div>
          <Popover open={userMenuOpen} onOpenChange={setUserMenuOpen}>
            <PopoverTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-auto max-w-[200px] gap-1.5 px-2 py-1.5 font-medium text-muted-foreground hover:text-foreground"
                aria-label={`账号菜单：${user?.username ?? 'guest'}`}
                aria-expanded={userMenuOpen}
              >
                <span className="min-w-0 flex-1 truncate text-left text-sm">
                  {user?.username ?? 'guest'}
                </span>
                <ChevronDown
                  className="size-4 shrink-0 text-muted-foreground"
                  aria-hidden
                />
              </Button>
            </PopoverTrigger>
            <PopoverContent align="end" className="w-48 p-1" sideOffset={6}>
              <div className="flex flex-col gap-0.5" role="menu" aria-label="账号">
                <button
                  type="button"
                  role="menuitem"
                  className={cn(
                    'w-full truncate rounded-md px-2 py-1.5 text-left text-sm outline-none transition-colors',
                    'hover:bg-accent hover:text-accent-foreground',
                    'focus-visible:ring-2 focus-visible:ring-ring/50',
                  )}
                  onClick={() => {
                    setUserMenuOpen(false)
                    setChangePasswordOpen(true)
                  }}
                >
                  修改密码
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className={cn(
                    'w-full truncate rounded-md px-2 py-1.5 text-left text-sm outline-none transition-colors',
                    'hover:bg-accent hover:text-accent-foreground',
                    'focus-visible:ring-2 focus-visible:ring-ring/50',
                  )}
                  onClick={handleLogout}
                >
                  退出
                </button>
              </div>
            </PopoverContent>
          </Popover>
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

      <ChangePasswordModal
        open={changePasswordOpen}
        onClose={() => setChangePasswordOpen(false)}
      />
    </div>
  )
}
