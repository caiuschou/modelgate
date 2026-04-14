import { ChevronDown, Monitor, Moon, Sun } from 'lucide-react'
import { useState } from 'react'
import { Link, NavLink, Outlet, useLocation, useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import { cn } from '@/lib/utils'
import { ChangePasswordModal } from '@/features/auth/components/change-password-modal'
import { HeaderLoginModal } from '@/features/auth/components/header-login-modal'
import { useMyTeams } from '@/features/teams/hooks/use-teams'
import { useAuthStore } from '@/stores/auth-store'
import { useTeamStore } from '@/stores/team-store'
import { useUiStore } from '@/stores/ui-store'

const themeChoices = [
  { value: 'light' as const, label: '浅色', Icon: Sun },
  { value: 'dark' as const, label: '深色', Icon: Moon },
  { value: 'system' as const, label: '跟随系统', Icon: Monitor },
]

type SidebarItem = { to: string; label: string; guestOk: boolean }

/** 侧栏分组：概览 → 接入与凭据 → 组织与协作 → 日志与分析 → 账户 → 系统 */
const sidebarGroups: { heading: string; items: SidebarItem[] }[] = [
  {
    heading: '接入与凭据',
    items: [
      { to: '/api-keys', label: 'API 密钥', guestOk: false },
      { to: '/byok-profiles', label: 'BYOK', guestOk: false },
    ],
  },
  {
    heading: '组织与协作',
    items: [
      { to: '/teams', label: '团队', guestOk: false },
      { to: '/users', label: '用户管理', guestOk: false },
    ],
  },
  {
    heading: '日志与分析',
    items: [
      { to: '/logs', label: '日志中心', guestOk: false },
      { to: '/logs/v2', label: '会话日志', guestOk: false },
      { to: '/analytics', label: '统计分析', guestOk: false },
    ],
  },
  {
    heading: '账户',
    items: [{ to: '/billing', label: '充值中心', guestOk: false }],
  },
  {
    heading: '系统',
    items: [{ to: '/settings', label: '系统设置', guestOk: false }],
  },
]

function sidebarHref(
  to: string,
  guestOk: boolean,
  token: string | null,
): string {
  if (token || guestOk) {
    return to
  }
  return `/login?redirect=${encodeURIComponent(to)}`
}

/** 顶栏「控制台」高亮：特性首页与模型页不算控制台。 */
function isConsoleTopNavActive(pathname: string): boolean {
  return (
    pathname !== '/' &&
    pathname !== '/models' &&
    !pathname.startsWith('/models/')
  )
}

function isSidebarLinkActive(to: string, pathname: string): boolean {
  if (to === '/') return pathname === '/'
  if (to === '/logs/v2') {
    return pathname === '/logs/v2'
  }
  if (to === '/logs') {
    if (pathname.startsWith('/logs/v2')) return false
    return pathname === '/logs' || pathname.startsWith('/logs/')
  }
  return pathname === to || pathname.startsWith(`${to}/`)
}

const topNavTabClass = (active: boolean) =>
  cn(
    'inline-flex h-8 shrink-0 items-center rounded-md border px-3 text-sm font-medium transition-colors',
    active
      ? 'border-transparent bg-secondary text-secondary-foreground'
      : 'border-dashed border-border text-foreground hover:bg-accent',
  )

export function AppLayout() {
  const location = useLocation()
  const navigate = useNavigate()
  const token = useAuthStore((state) => state.token)
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
  const [loginModalOpen, setLoginModalOpen] = useState(false)

  const homeSidebarItem = token
    ? { to: '/dashboard' as const, label: '首页', guestOk: false as const }
    : { to: '/' as const, label: '首页', guestOk: true as const }
  /** 访客特性首页、模型目录页与控制台分离：不展示左侧控制台菜单。 */
  const guestMarketingHome = location.pathname === '/' && !token
  const modelsStandaloneRoute =
    location.pathname === '/models' ||
    location.pathname.startsWith('/models/')
  const showConsoleSidebar = !guestMarketingHome && !modelsStandaloneRoute
  const teams = teamsRes?.data ?? []
  const currentSpaceLabel =
    currentTeamId == null
      ? '个人空间'
      : (teams.find((t) => t.id === currentTeamId)?.name ??
        `团队 #${currentTeamId}`)

  const handleLogout = () => {
    setUserMenuOpen(false)
    logout()
    navigate('/')
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="sticky top-0 z-50 flex h-14 items-center justify-between border-b border-border bg-card px-4">
        <div className="flex min-w-0 flex-1 items-center gap-3">
          {showConsoleSidebar ? (
            <Button variant="outline" size="sm" onClick={toggleSidebar}>
              {sidebarCollapsed ? '展开' : '折叠'}
            </Button>
          ) : null}
          <span className="shrink-0 font-semibold">ModelGate Console</span>
          <nav
            className="flex shrink-0 items-center gap-2"
            aria-label="顶层导航"
          >
            {token ? (
              <Link
                to="/dashboard"
                className={topNavTabClass(
                  isConsoleTopNavActive(location.pathname),
                )}
              >
                控制台
              </Link>
            ) : (
              <button
                type="button"
                className={topNavTabClass(false)}
                onClick={() => setLoginModalOpen(true)}
              >
                控制台
              </button>
            )}
            <NavLink
              to="/models"
              className={({ isActive }) => topNavTabClass(isActive)}
            >
              模型
            </NavLink>
          </nav>
        </div>
        <div className="flex items-center gap-3 text-sm">
          {token ? (
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
          ) : (
            <span
              className="max-w-[220px] truncate text-muted-foreground sm:max-w-[280px]"
              title="登录后可切换个人空间与团队"
            >
              访客模式
            </span>
          )}
          {!token ? (
            <>
              <Button
                type="button"
                size="sm"
                onClick={() => setLoginModalOpen(true)}
              >
                登录
              </Button>
              <Button variant="outline" size="sm" asChild>
                <Link to="/register">注册</Link>
              </Button>
            </>
          ) : null}
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
          {token ? (
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
          ) : null}
        </div>
      </header>

      <div
        className={cn(
          'mx-auto flex w-full min-w-0 max-w-[1400px]',
          !showConsoleSidebar && 'max-w-[1200px]',
        )}
      >
        {showConsoleSidebar ? (
          <aside
            className={`border-r border-border bg-card p-3 ${sidebarCollapsed ? 'w-20' : 'w-56'}`}
          >
            <nav className="space-y-1" aria-label="控制台侧栏">
              <div className="space-y-1">
                {!sidebarCollapsed ? (
                  <div className="px-3 pb-1 text-[11px] font-semibold tracking-wide text-muted-foreground">
                    概览
                  </div>
                ) : null}
                {(() => {
                  const item = homeSidebarItem
                  const href = sidebarHref(item.to, item.guestOk, token)
                  const isActive = isSidebarLinkActive(item.to, location.pathname)
                  return (
                    <Link
                      key={`${item.to}-home`}
                      to={href}
                      className={`block rounded px-3 py-2 text-sm ${
                        isActive
                          ? 'bg-primary text-primary-foreground'
                          : 'text-muted-foreground hover:bg-accent hover:text-foreground'
                      }`}
                    >
                      {sidebarCollapsed ? item.label.slice(0, 2) : item.label}
                    </Link>
                  )
                })()}
              </div>
              {sidebarGroups.map((group, gi) => (
                <div
                  key={group.heading}
                  className={cn(
                    'space-y-1',
                    gi > 0 && 'mt-3 border-t border-border/70 pt-3',
                  )}
                >
                  {!sidebarCollapsed ? (
                    <div className="px-3 pb-1 text-[11px] font-semibold tracking-wide text-muted-foreground">
                      {group.heading}
                    </div>
                  ) : null}
                  {group.items.map((item) => {
                    const href = sidebarHref(item.to, item.guestOk, token)
                    const isActive = isSidebarLinkActive(
                      item.to,
                      location.pathname,
                    )
                    return (
                      <Link
                        key={`${item.to}-${item.label}`}
                        to={href}
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
                </div>
              ))}
            </nav>
          </aside>
        ) : null}

        <main
          className={cn(
            'min-h-[calc(100vh-56px)] min-w-0 flex-1 p-6',
            !showConsoleSidebar && 'w-full',
          )}
        >
          <Outlet />
        </main>
      </div>

      <ChangePasswordModal
        open={changePasswordOpen}
        onClose={() => setChangePasswordOpen(false)}
      />

      <HeaderLoginModal
        open={loginModalOpen}
        onClose={() => setLoginModalOpen(false)}
        onLoggedIn={() => navigate('/dashboard')}
      />
    </div>
  )
}
