import { expect, test } from '@playwright/test'
import {
  consoleSidebarNav,
  sidebarTriggerFromHeader,
} from './helpers/console-nav'

test.describe('控制台布局与侧栏', () => {
  test('仪表盘页展示控制台侧栏与切换按钮', async ({ page }) => {
    await page.goto('/dashboard')
    await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
    await expect(consoleSidebarNav(page)).toBeVisible()
    await expect(sidebarTriggerFromHeader(page)).toBeVisible()
    await expect(
      consoleSidebarNav(page).getByRole('link', { name: '首页' }),
    ).toBeVisible()
    await expect(
      consoleSidebarNav(page).getByRole('link', { name: 'API 密钥' }),
    ).toBeVisible()
  })

  test('侧栏可进入 API 密钥页', async ({ page }) => {
    await page.goto('/dashboard')
    await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
    await consoleSidebarNav(page).getByRole('link', { name: 'API 密钥' }).click()
    await expect(page).toHaveURL(/\/api-keys$/)
    await expect(page.getByRole('heading', { name: 'API 密钥' })).toBeVisible()
  })

  test('折叠侧栏后仍可通过链接名称进入会话日志', async ({ page }) => {
    await page.goto('/dashboard')
    await sidebarTriggerFromHeader(page).click()
    await consoleSidebarNav(page).getByRole('link', { name: '会话日志' }).click()
    await expect(page).toHaveURL(/\/logs\/v2$/)
  })

  test('模型目录页不展示控制台侧栏', async ({ page }) => {
    await page.goto('/models')
    await expect(consoleSidebarNav(page)).toHaveCount(0)
    await expect(sidebarTriggerFromHeader(page)).toHaveCount(0)
  })
})

test.describe('访客布局', () => {
  test.use({ storageState: { cookies: [], origins: [] } })

  test('营销首页不展示控制台侧栏', async ({ page }) => {
    await page.goto('/')
    await expect(consoleSidebarNav(page)).toHaveCount(0)
    await expect(sidebarTriggerFromHeader(page)).toHaveCount(0)
  })
})
