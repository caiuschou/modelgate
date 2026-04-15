import type { LucideIcon } from 'lucide-react'
import {
  BarChart3,
  Building2,
  CreditCard,
  FileText,
  Home,
  Key,
  LayoutDashboard,
  MessageSquare,
  Settings,
  Shield,
  UserCog,
} from 'lucide-react'

export type ConsoleNavItem = {
  to: string
  label: string
  guestOk: boolean
  icon: LucideIcon
}

export type ConsoleNavGroup = {
  heading: string
  items: ConsoleNavItem[]
}

/** 侧栏分组：接入与凭据 → 组织与协作 → 日志与分析 → 账户 → 系统（「概览」在布局里单独渲染首页） */
export const consoleNavGroups: ConsoleNavGroup[] = [
  {
    heading: '接入与凭据',
    items: [
      { to: '/api-keys', label: 'API 密钥', guestOk: false, icon: Key },
      { to: '/byok-profiles', label: 'BYOK', guestOk: false, icon: Shield },
    ],
  },
  {
    heading: '组织与协作',
    items: [
      { to: '/teams', label: '团队', guestOk: false, icon: Building2 },
      { to: '/users', label: '用户管理', guestOk: false, icon: UserCog },
    ],
  },
  {
    heading: '日志与分析',
    items: [
      { to: '/logs', label: '日志中心', guestOk: false, icon: FileText },
      { to: '/logs/v2', label: '会话日志', guestOk: false, icon: MessageSquare },
      { to: '/analytics', label: '统计分析', guestOk: false, icon: BarChart3 },
    ],
  },
  {
    heading: '账户',
    items: [
      { to: '/billing', label: '充值中心', guestOk: false, icon: CreditCard },
    ],
  },
  {
    heading: '系统',
    items: [{ to: '/settings', label: '系统设置', guestOk: false, icon: Settings }],
  },
]

export function getHomeNavItem(token: string | null): ConsoleNavItem {
  return token
    ? {
        to: '/dashboard',
        label: '首页',
        guestOk: false,
        icon: LayoutDashboard,
      }
    : { to: '/', label: '首页', guestOk: true, icon: Home }
}

export function sidebarHref(
  to: string,
  guestOk: boolean,
  token: string | null,
): string {
  if (token || guestOk) {
    return to
  }
  return `/login?redirect=${encodeURIComponent(to)}`
}

export function isSidebarLinkActive(to: string, pathname: string): boolean {
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
