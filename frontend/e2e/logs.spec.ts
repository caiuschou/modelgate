import { expect, test } from '@playwright/test'
import {
  createChatCompletion,
  getGatewayApiKeyForSession,
  loginApiKey,
  waitForAuditListRow,
} from './helpers/api'

const consoleBase = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:3000'
const backendBase = process.env.E2E_BACKEND_URL ?? 'http://127.0.0.1:8000'
const e2eUser = process.env.E2E_USERNAME ?? 'e2e_user'
const e2ePass = process.env.E2E_PASSWORD ?? 'E2e_local_pass_1'

function unixNow(): number {
  return Math.floor(Date.now() / 1000)
}

test('log center heading is visible when authenticated', async ({ page }) => {
  await page.goto('/logs')
  await expect(page.getByRole('heading', { name: '日志中心' })).toBeVisible()
})

test('list shows audit row after chat completion and opens detail', async ({
  page,
}) => {
  const model = `e2e_audit_${Date.now()}`
  const appId = `e2e_app_${Date.now()}`
  const session = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const gatewayKey = await getGatewayApiKeyForSession(consoleBase, session)
  const chat = await createChatCompletion(backendBase, gatewayKey, model, {
    appId,
  })
  expect(chat.ok, `chat completions failed: ${await chat.text()}`).toBeTruthy()

  const end = unixNow() + 3600
  const row = await waitForAuditListRow(backendBase, session, {
    start_time: '0',
    end_time: String(end),
    limit: '20',
    offset: '0',
    model,
  })
  expect(row, 'audit row did not appear (flush timeout)').not.toBeNull()

  await page.goto('/logs')
  const dataRow = page.getByRole('row').filter({ hasText: model })
  await expect(dataRow).toBeVisible({ timeout: 20_000 })
  await expect(dataRow).toContainText(appId)

  await dataRow.getByRole('link', { name: '详情' }).click()

  await expect(page.getByRole('heading', { name: '日志详情' })).toBeVisible()
  await expect(
    page.locator('dt', { hasText: 'request_id' }).locator('+ dd'),
  ).toHaveText(row!.request_id)
  await expect(
    page.locator('dt', { hasText: '模型' }).locator('+ dd'),
  ).toHaveText(model)
  await expect(
    page.locator('dt', { hasText: '应用 (app_id)' }).locator('+ dd'),
  ).toHaveText(appId)
  await expect(
    page.locator('dt', { hasText: 'Finish 原因' }).locator('+ dd'),
  ).toHaveText('stop')
})

test('keyword in URL shows matching audit row', async ({ page }) => {
  const model = `e2e_kw_${Date.now()}`
  const session = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const gatewayKey = await getGatewayApiKeyForSession(consoleBase, session)
  const chat = await createChatCompletion(backendBase, gatewayKey, model)
  expect(chat.ok, `chat completions failed: ${await chat.text()}`).toBeTruthy()

  const end = unixNow() + 3600
  const row = await waitForAuditListRow(backendBase, session, {
    start_time: '0',
    end_time: String(end),
    limit: '20',
    offset: '0',
    model,
  })
  expect(row).not.toBeNull()
  const kw = row!.request_id.slice(0, 8)
  await page.goto(
    `/logs?keyword=${encodeURIComponent(kw)}&start_time=0&end_time=${end}&offset=0`,
  )
  await expect(page.getByRole('row').filter({ hasText: row!.request_id })).toBeVisible({
    timeout: 20_000,
  })
})

test('model filter syncs to URL query when applying filters', async ({
  page,
}) => {
  await page.goto('/logs')
  const model = `e2e_filter_${Date.now()}`
  await page.getByLabel('模型').fill(model)
  await page.getByRole('button', { name: '查询' }).click()
  await expect(page).toHaveURL(new RegExp(`[?&]model=${encodeURIComponent(model)}`))
})

test('export CSV downloads a file', async ({ page }) => {
  const model = `e2e_export_${Date.now()}`
  const session = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const gatewayKey = await getGatewayApiKeyForSession(consoleBase, session)
  const chat = await createChatCompletion(backendBase, gatewayKey, model)
  expect(chat.ok, `chat completions failed: ${await chat.text()}`).toBeTruthy()

  const end = unixNow() + 3600
  const row = await waitForAuditListRow(backendBase, session, {
    start_time: '0',
    end_time: String(end),
    limit: '20',
    offset: '0',
    model,
  })
  expect(row).not.toBeNull()

  await page.goto('/logs')
  await expect(page.getByRole('row').filter({ hasText: model })).toBeVisible({
    timeout: 20_000,
  })

  const downloadPromise = page.waitForEvent('download', { timeout: 60_000 })
  await page.getByRole('button', { name: '导出 CSV' }).click()
  const download = await downloadPromise
  expect(download.suggestedFilename()).toMatch(/\.csv$/i)
})
