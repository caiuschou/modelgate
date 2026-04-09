import { expect, test } from '@playwright/test'

/**
 * Billing UI: authenticated console + optional hidden admin top-up route (dev default segment).
 * Backend E2E stack keeps `billing.enabled = false` so chat flows stay unblocked; these tests only cover pages.
 */

test('billing center shows balance area and ledger tabs', async ({ page }) => {
  await page.goto('/billing')
  await expect(page.getByRole('heading', { name: '充值中心' })).toBeVisible()
  await expect(page.getByText('当前余额')).toBeVisible()

  await page.getByRole('button', { name: '消费记录' }).click()
  await expect(
    page.getByText('暂无模型调用扣费记录。'),
  ).toBeVisible({ timeout: 15_000 })

  await page.getByRole('button', { name: '充值记录' }).click()
  await expect(page.getByText('暂无充值记录。')).toBeVisible({
    timeout: 15_000,
  })
})

test('hidden admin recharge page is reachable', async ({ page }) => {
  await page.goto('/__mg-admin-recharge')
  // CardTitle is a styled div, not a semantic heading
  await expect(page.getByText('管理充值', { exact: true })).toBeVisible()
  await expect(page.getByLabel('控制台用户名')).toBeVisible()
  await expect(page.getByLabel('充值金额（USD）')).toBeVisible()
  await expect(page.getByLabel('管理密码')).toBeVisible()
  await expect(
    page.getByRole('button', { name: '确认充值' }),
  ).toBeVisible()
})
