import { expect, test } from '@playwright/test'
import { loadE2eSessionCredentials } from './load-e2e-credentials'

test.use({ storageState: { cookies: [], origins: [] } })

test('session credentials work via HTTP (same stack as UI)', async ({ request }) => {
  const { username, password } = loadE2eSessionCredentials()
  const res = await request.post('/api/v1/auth/login', {
    data: { username, password },
  })
  expect(res.status(), await res.text()).toBe(200)
})

test('login shows dashboard', async ({ page }) => {
  const { username, password } = loadE2eSessionCredentials()

  await page.goto('/login')
  await page.getByLabel('用户名').fill(username)
  await page.getByLabel('密码').fill(password)
  await expect(page.getByLabel('用户名')).toHaveValue(username)
  await expect(page.getByLabel('密码')).toHaveValue(password)
  await Promise.all([
    page.waitForURL((url) => url.pathname === '/dashboard', { timeout: 20_000 }),
    page.getByRole('button', { name: '登录' }).click(),
  ])
  await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
})
