import { expect, test } from '@playwright/test'

test('dashboard loads chart sections for authenticated user', async ({ page }) => {
  await page.goto('/dashboard')
  await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
  await expect(
    page.getByText('近 24 小时 · 按小时成本 (USD)'),
  ).toBeVisible({ timeout: 15_000 })
  await expect(page.getByText('近 24 小时 · Token（单柱堆叠）')).toBeVisible()
  await expect(page.getByText(/固定 24 个整点小时/)).toBeVisible()
  await expect(page.getByText(/单柱堆叠/)).toBeVisible()
  await expect(page.getByText('近 24h Token 合计')).toBeVisible()
  await expect(page.getByText('近 24h 成本 (USD)')).toBeVisible()
})
