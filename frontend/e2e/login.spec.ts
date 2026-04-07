import { expect, test } from '@playwright/test'
import { loadE2eSessionCredentials } from './load-e2e-credentials'

test.use({ storageState: { cookies: [], origins: [] } })

test('login shows dashboard', async ({ page }) => {
  const { username, password } = loadE2eSessionCredentials()

  await page.goto('/login')
  await page.locator('input[name="username"]').fill(username)
  await page.locator('input[name="password"]').fill(password)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
})
