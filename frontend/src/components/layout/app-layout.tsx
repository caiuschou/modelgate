import { ChevronDown, Monitor, Moon, Sun } from 'lucide-react'
import { useState } from 'react'
import { Link, NavLink, Outlet, useLocation, useNavigate } from 'react-router-dom'
import { ConsoleSidebar } from '@/components/layout/console-sidebar'
import { Button } from '@/components/ui/button'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from '@/components/ui/sidebar'
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

/** 顶栏「控制台」高亮：特性首页与模型页不算控制台。 */
function isConsoleTopNavActive(pathname: string): boolean {
  return (
    pathname !== '/' &&
    pathname !== '/models' &&
    !pathname.startsWith('/models/')
  )
}

const topNavTabClass = (active: boolean) =>
  cn(
    'inline-flex h-8 shrink-0 items-center rounded-md border px-3 text-sm font-medium transition-colors',
    active
      ? 'border-transparent bg-secondary text-secondary-foreground'
      : 'border-dashed border-border text-foreground hover:bg-accent',
  )

function AppShellHeader({ showSidebarTrigger }: { showSidebarTrigger: boolean }) {
  const location = useLocation()
  const navigate = useNavigate()
  const token = useAuthStore((state) => state.token)
  const user = useAuthStore((state) => state.user)
  const logout = useAuthStore((state) => state.logout)
  const currentTeamId = useTeamStore((s) => s.currentTeamId)
  const setTeamContext = useTeamStore((s) => s.setTeamContext)
  const { data: teamsRes } = useMyTeams()
  const theme = useUiStore((state) => state.theme)
  const setTheme = useUiStore((state) => state.setTheme)
  const [workspaceOpen, setWorkspaceOpen] = useState(false)
  const [userMenuOpen, setUserMenuOpen] = useState(false)
  const [changePasswordOpen, setChangePasswordOpen] = useState(false)
  const [loginModalOpen, setLoginModalOpen] = useState(false)

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
    <>
      <header className="sticky top-0 z-50 flex h-14 shrink-0 items-center justify-between border-b border-border bg-card px-4">
        <div className="flex min-w-0 flex-1 items-center gap-2 sm:gap-3">
          {showSidebarTrigger ? (
            <SidebarTrigger
              aria-label="切换侧栏"
              title="切换侧栏（Ctrl+B）"
              className="-ms-1"
            />
          ) : null}
          <span className="shrink-0 font-semibold">Model Gate Console</span>
          <nav
            className="flex min-w-0 shrink-0 items-center gap-2"
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
        <div className="flex items-center gap-2 text-sm sm:gap-3">
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
                <div
                  className="flex flex-col gap-0.5"
                  role="menu"
                  aria-label="账号"
                >
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

      <ChangePasswordModal
        open={changePasswordOpen}
        onClose={() => setChangePasswordOpen(false)}
      />

      <HeaderLoginModal
        open={loginModalOpen}
        onClose={() => setLoginModalOpen(false)}
        onLoggedIn={() => navigate('/dashboard')}
      />
    </>
  )
}

export function AppLayout() {
  const location = useLocation()
  const token = useAuthStore((state) => state.token)
  const sidebarCollapsed = useUiStore((state) => state.sidebarCollapsed)
  const setSidebarExpanded = useUiStore((state) => state.setSidebarExpanded)

  const guestMarketingHome = location.pathname === '/' && !token
  const modelsStandaloneRoute =
    location.pathname === '/models' ||
    location.pathname.startsWith('/models/')
  const showConsoleSidebar = !guestMarketingHome && !modelsStandaloneRoute

  return (
    <>
      {showConsoleSidebar ? (
        <SidebarProvider
          open={!sidebarCollapsed}
          onOpenChange={setSidebarExpanded}
          className="min-h-svh w-full"
        >
          <ConsoleSidebar />
          <SidebarInset>
            <AppShellHeader showSidebarTrigger />
            <div
              className={cn(
                'mx-auto min-h-0 w-full min-w-0 flex-1 p-6 max-w-[1400px]',
              )}
            >
              <Outlet />
            </div>
          </SidebarInset>
        </SidebarProvider>
      ) : (
        <div className="flex min-h-svh flex-col bg-background text-foreground">
          <AppShellHeader showSidebarTrigger={false} />
          <div
            className={cn(
              'mx-auto w-full min-w-0 flex-1 p-6 max-w-[1200px]',
            )}
          >
            <Outlet />
          </div>
        </div>
      )}
    </>
  )
}
