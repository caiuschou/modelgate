import { expect, test } from '@playwright/test'

test.use({ storageState: { cookies: [], origins: [] } })

test('login shows dashboard', async ({ page }) => {
  const username = process.env.E2E_USERNAME ?? 'e2e_user'
  const password = process.env.E2E_PASSWORD ?? 'E2e_local_pass_1'

  await page.goto('/login')
  await page.getByLabel('用户名').fill(username)
  await page.getByLabel('密码').fill(password)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
})
