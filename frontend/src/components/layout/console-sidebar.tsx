import { Link, useLocation } from 'react-router-dom'
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  SidebarSeparator,
} from '@/components/ui/sidebar'
import { useAuthStore } from '@/stores/auth-store'
import {
  consoleNavGroups,
  getHomeNavItem,
  isSidebarLinkActive,
  sidebarHref,
  type ConsoleNavItem,
} from './sidebar-nav-config'

function NavItemLink({
  item,
  token,
  pathname,
}: {
  item: ConsoleNavItem
  token: string | null
  pathname: string
}) {
  const href = sidebarHref(item.to, item.guestOk, token)
  const active = isSidebarLinkActive(item.to, pathname)
  const Icon = item.icon

  return (
    <SidebarMenuItem>
      <SidebarMenuButton asChild isActive={active} tooltip={item.label}>
        <Link to={href} aria-current={active ? 'page' : undefined}>
          <Icon aria-hidden />
          <span>{item.label}</span>
        </Link>
      </SidebarMenuButton>
    </SidebarMenuItem>
  )
}

export function ConsoleSidebar() {
  const token = useAuthStore((s) => s.token)
  const pathname = useLocation().pathname
  const homeItem = getHomeNavItem(token)

  return (
    <Sidebar collapsible="icon" variant="inset">
      <SidebarHeader className="border-b border-sidebar-border">
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton size="lg" asChild tooltip="ModelGate 控制台">
              <Link to={token ? '/dashboard' : '/'}>
                <span className="truncate font-semibold">MG</span>
                <span className="truncate text-xs text-sidebar-foreground/80">
                  ModelGate
                </span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        <nav
          aria-label="控制台侧栏"
          className="flex min-h-0 flex-1 flex-col gap-0"
        >
          <SidebarGroup>
            <SidebarGroupLabel>概览</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                <NavItemLink
                  item={homeItem}
                  token={token}
                  pathname={pathname}
                />
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
          {consoleNavGroups.map((group) => (
            <SidebarGroup key={group.heading}>
              <SidebarSeparator className="my-2" />
              <SidebarGroupLabel>{group.heading}</SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  {group.items.map((item) => (
                    <NavItemLink
                      key={`${item.to}-${item.label}`}
                      item={item}
                      token={token}
                      pathname={pathname}
                    />
                  ))}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          ))}
        </nav>
      </SidebarContent>
      <SidebarRail />
    </Sidebar>
  )
}
