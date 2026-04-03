import { expect, test } from '@playwright/test'

test.use({ storageState: { cookies: [], origins: [] } })

test('unauthenticated visit to /logs redirects to login with redirect query', async ({
  page,
}) => {
  await page.goto('/logs')
  await expect(page).toHaveURL(/\/login\?redirect=/)
})
