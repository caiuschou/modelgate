import { expect, test } from '@playwright/test'
import {
  createChatCompletion,
  createChatCompletionStream,
  getGatewayApiKeyForSession,
  loginApiKey,
  waitForAuditDetailResponsePath,
  waitForAuditListRow,
} from './helpers/api'
import { loadE2eSessionCredentials } from './load-e2e-credentials'

const consoleBase = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:3000'
const backendBase = process.env.E2E_BACKEND_URL ?? 'http://127.0.0.1:8000'

let e2eUser: string
let e2ePass: string

test.beforeAll(() => {
  const c = loadE2eSessionCredentials()
  e2eUser = c.username
  e2ePass = c.password
})

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

test('detail page shows request and response body from audit files', async ({
  page,
}) => {
  const model = `e2e_body_${Date.now()}`
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
  expect(row, 'audit row did not appear (flush timeout)').not.toBeNull()

  await page.goto('/logs')
  await expect(page.getByRole('row').filter({ hasText: model })).toBeVisible({
    timeout: 20_000,
  })
  await page.getByRole('row').filter({ hasText: model }).getByRole('link', { name: '详情' }).click()

  await expect(page.getByRole('heading', { name: '日志详情' })).toBeVisible()
  await expect(page.getByRole('heading', { name: '请求体' })).toBeVisible()
  await expect(page.getByRole('heading', { name: '响应体' })).toBeVisible()

  // Bodies render in `VirtualizedLogBody`; scope to panels so metadata `<dd>` does not collide.
  const requestSection = page.getByRole('heading', { name: '请求体' }).locator('..')
  const responseSection = page.getByRole('heading', { name: '响应体' }).locator('..')
  await expect(requestSection.getByText(model, { exact: false })).toBeVisible({
    timeout: 20_000,
  })
  await expect(
    requestSection.getByText('e2e audit ping', { exact: false }),
  ).toBeVisible({ timeout: 20_000 })
  await expect(responseSection.getByText('choices', { exact: false })).toBeVisible({
    timeout: 20_000,
  })
})

test('stream chat persists SSE and detail shows body', async ({ page }) => {
  const model = `e2e_stream_${Date.now()}`
  const session = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const gatewayKey = await getGatewayApiKeyForSession(consoleBase, session)
  const streamResp = await createChatCompletionStream(backendBase, gatewayKey, model)
  const streamText = await streamResp.text()
  expect(streamResp.ok, `stream chat failed: ${streamText}`).toBeTruthy()

  const end = unixNow() + 3600
  const row = await waitForAuditListRow(backendBase, session, {
    start_time: '0',
    end_time: String(end),
    limit: '20',
    offset: '0',
    model,
  })
  expect(row).not.toBeNull()

  const okPath = await waitForAuditDetailResponsePath(
    backendBase,
    session,
    row!.request_id,
  )
  expect(okPath, 'stream response_body_path not set after completion').toBe(true)

  await page.goto(`/logs/${encodeURIComponent(row!.request_id)}`)
  await expect(page.getByRole('heading', { name: '日志详情' })).toBeVisible()
  await expect(
    page.getByText('e2e stream chunk', { exact: false }),
  ).toBeVisible({ timeout: 20_000 })
  await expect(page.getByText('[DONE]', { exact: false })).toBeVisible()
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
