import type { Page } from '@playwright/test'

/** 控制台左侧导航（`aria-label="控制台侧栏"`） */
export function consoleSidebarNav(page: Page) {
  return page.getByRole('navigation', { name: '控制台侧栏' })
}

/**
 * 顶栏侧栏触发器（`title` 与 `SidebarRail` 区分，避免与「切换侧栏」重名导致 strict mode）。
 */
export function sidebarTriggerFromHeader(page: Page) {
  return page.getByTitle('切换侧栏（Ctrl+B）')
}
