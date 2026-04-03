import { expect, test } from '@playwright/test'

/** 整文件无登录态；与 `auth-guard.spec.ts` 相同模式（describe 内 test.use 无法覆盖 config 的 storageState） */
test.use({ storageState: { cookies: [], origins: [] } })

test('未登录访问 /api-keys 会跳到登录页', async ({ page }) => {
  await page.goto('/api-keys')
  await expect(page).toHaveURL(/\/login/)
  await expect(page.getByText('登录 ModelGate')).toBeVisible()
})
