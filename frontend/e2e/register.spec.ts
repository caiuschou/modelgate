import { expect, test } from '@playwright/test'

test.use({ storageState: { cookies: [], origins: [] } })

test('register then land on login with username prefilled', async ({ page }) => {
  const invite = process.env.E2E_INVITE_CODE ?? 'e2e-invite-code'
  const username = `e2e_reg_${Date.now()}`
  const password = 'E2e_reg_pass_1'

  await page.goto('/register')
  await page.locator('input[name="username"]').fill(username)
  await page.locator('input[name="password"]').fill(password)
  await page.locator('input[name="invite_code"]').fill(invite)

  await page.getByRole('button', { name: '注册' }).click()
  await expect(page).toHaveURL(new RegExp(`/login\\?username=${encodeURIComponent(username)}`))
})
