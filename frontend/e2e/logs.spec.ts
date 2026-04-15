import { expect, test, type Page } from '@playwright/test'
import {
  createChatCompletion,
  createChatCompletionStream,
  getGatewayApiKeyForSession,
  loginApiKey,
  waitForAuditDetailResponsePath,
  waitForAuditListRow,
  waitForAuditThreadListRow,
} from './helpers/api'
import { loadE2eSessionCredentials } from './load-e2e-credentials'

const consoleBase = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:5173'
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

/** 与日志列表页右侧详情 Sheet 的 `width: calc(100vw * 2 / 3)` 一致（允许少量子像素 / 边框误差）。 */
async function expectLogDetailSheetApproxTwoThirdsViewport(page: Page) {
  const viewport = page.viewportSize()
  expect(viewport).not.toBeNull()
  const panel = page.locator('[data-slot="sheet-content"]')
  await expect(panel).toBeVisible()
  const box = await panel.boundingBox()
  expect(box).not.toBeNull()
  const expectedWidth = (viewport!.width * 2) / 3
  expect(
    Math.abs(box!.width - expectedWidth),
    `sheet width ${box!.width}px vs expected ~${expectedWidth}px (2/3 of ${viewport!.width}px viewport)`,
  ).toBeLessThan(8)
}

test('log center heading is visible when authenticated', async ({ page }) => {
  await page.goto('/logs')
  await expect(page.getByRole('heading', { name: '日志中心' })).toBeVisible()
})

test('log center v2 list heading is visible when authenticated', async ({
  page,
}) => {
  await page.goto('/logs/v2')
  await expect(
    page.getByRole('heading', { name: '会话中心' }),
  ).toBeVisible()
  // 无数据时不渲染表头；会话中心默认收起整块筛选，仅保留摘要条与查询
  await expect(
    page.getByRole('button', { name: '展开筛选条件' }),
  ).toBeVisible()
  await expect(page.getByRole('button', { name: '查询' })).toBeVisible()
})

test('log center v2 shows session row and links to classic list with thread filter', async ({
  page,
}) => {
  const model = `e2e_v2_thread_${Date.now()}`
  const threadId = `e2e_sess_${Date.now()}`
  const session = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const gatewayKey = await getGatewayApiKeyForSession(consoleBase, session)
  const chat = await createChatCompletion(backendBase, gatewayKey, model, {
    threadId,
  })
  expect(chat.ok, `chat completions failed: ${await chat.text()}`).toBeTruthy()

  const end = unixNow() + 3600
  const thread = await waitForAuditThreadListRow(backendBase, session, {
    start_time: '0',
    end_time: String(end),
    limit: '20',
    offset: '0',
    thread_id: threadId,
  })
  expect(thread, 'threads API did not list session (flush timeout)').not.toBeNull()
  expect(thread!.thread_id).toBe(threadId)
  expect(thread!.request_count).toBeGreaterThanOrEqual(1)
  expect(thread!.last_prompt_preview).toContain('e2e audit ping')

  await page.goto('/logs/v2')
  await expect(
    page.getByRole('columnheader', { name: '活动时间' }),
  ).toBeVisible({ timeout: 25_000 })
  const dataRow = page.getByRole('row').filter({ hasText: threadId })
  await expect(dataRow).toBeVisible({ timeout: 25_000 })
  await expect(dataRow.getByText('e2e audit ping')).toBeVisible()
  // 列：展开 · 活动时间 · 会话(thread+摘要) · 请求数 · …
  await expect(dataRow.getByRole('cell').nth(3)).toHaveText(
    String(thread!.request_count),
  )

  await dataRow
    .getByRole('button', { name: '查看会话下的请求' })
    .click()
  const threadRequestsRegion = page.getByRole('region', {
    name: '会话下的请求',
  })
  await expect(threadRequestsRegion).toBeVisible({ timeout: 15_000 })
  await expect(
    threadRequestsRegion.getByRole('button', { name: '查看请求详情' }),
  ).toBeVisible()

  await threadRequestsRegion
    .getByRole('link', { name: '在经典列表中打开' })
    .click()
  await expect(page).toHaveURL(/\/logs\?/)
  const url = new URL(page.url())
  expect(url.searchParams.get('thread_id')).toBe(threadId)

  await expect(page.getByRole('heading', { name: '日志中心' })).toBeVisible()
  await expect(
    page.getByRole('heading', { name: '会话中心' }),
  ).not.toBeVisible()
  await expect(page.getByRole('row').filter({ hasText: model })).toBeVisible({
    timeout: 20_000,
  })
})

test('log list token cells show usage tooltip on hover', async ({ page }) => {
  const model = `e2e_tip_${Date.now()}`
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
  const dataRow = page.getByRole('row').filter({ hasText: model })
  await expect(dataRow).toBeVisible({ timeout: 20_000 })
  // Columns: 时间, request_id, 模型, 应用, 会话, 状态, prompt, completion, …
  await dataRow.locator('td').nth(6).hover()
  await expect(page.locator('[role="tooltip"]')).toContainText('用量与计费', {
    timeout: 15_000,
  })
  await expect(page.locator('[role="tooltip"]')).toContainText('输入（prompt）')
})

test('list shows audit row after chat completion and opens detail', async ({
  page,
}) => {
  const model = `e2e_audit_${Date.now()}`
  const appId = `e2e_app_${Date.now()}`
  const threadId = `e2e_thread_${Date.now()}`
  const session = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const gatewayKey = await getGatewayApiKeyForSession(consoleBase, session)
  const chat = await createChatCompletion(backendBase, gatewayKey, model, {
    appId,
    threadId,
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
  await expect(dataRow).toContainText(threadId)

  await dataRow.getByRole('button', { name: '查看请求详情' }).click()

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
    page.locator('dt', { hasText: '会话 (thread_id)' }).locator('+ dd'),
  ).toHaveText(threadId)
  await expect(
    page.locator('dt', { hasText: 'Finish 原因' }).locator('+ dd'),
  ).toHaveText('stop')

  await expect(page.getByRole('heading', { name: '请求头' })).toBeVisible()
  await expect(
    page.getByRole('heading', { name: '响应头（上游）' }),
  ).toBeVisible()
  await expect(page.getByRole('heading', { name: '请求头' }).locator('..')).toContainText(
    'content-type',
  )
  await expect(page.getByRole('heading', { name: '请求头' }).locator('..')).toContainText(
    'x-app-id',
  )
  await expect(page.getByRole('heading', { name: '请求头' }).locator('..')).toContainText(
    'x-thread-id',
  )
})

test('log list detail sheet width is about two thirds of viewport', async ({
  page,
}) => {
  const model = `e2e_sheet_w_${Date.now()}`
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
  const dataRow = page.getByRole('row').filter({ hasText: model })
  await expect(dataRow).toBeVisible({
    timeout: 20_000,
  })
  await dataRow.getByRole('button', { name: '查看请求详情' }).click()
  await expect(page.getByRole('heading', { name: '日志详情' })).toBeVisible()
  await expectLogDetailSheetApproxTwoThirdsViewport(page)
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
  await page
    .getByRole('row')
    .filter({ hasText: model })
    .getByRole('button', { name: '查看请求详情' })
    .click()

  await expect(page.getByRole('heading', { name: '日志详情' })).toBeVisible()
  const requestSection = page.getByTestId('audit-body-request')
  const responseSection = page.getByTestId('audit-body-response')
  await expect(requestSection).toBeVisible()
  await expect(responseSection).toBeVisible()
  await requestSection.locator(':scope > summary').click()
  await responseSection.locator(':scope > summary').click()

  // Bodies render in `VirtualizedLogBody`; scope to panels so metadata `<dd>` does not collide.
  // Parsed chat completion adds a collapsible raw body (`<details>`); expand before asserting on JSON.
  const rawSummaryRequest = requestSection.getByText('原始正文', { exact: true })
  if ((await rawSummaryRequest.count()) > 0) {
    await rawSummaryRequest.click()
  }
  const requestModelLine = requestSection.getByText(model, { exact: false }).first()
  await requestModelLine.scrollIntoViewIfNeeded()
  await expect(requestModelLine).toBeVisible({
    timeout: 20_000,
  })
  // Structured "内容" and raw JSON both contain the user message string.
  await expect(
    requestSection.getByText('e2e audit ping', { exact: true }).first(),
  ).toBeVisible({ timeout: 20_000 })
  const rawSummaryResponse = responseSection.getByText('原始正文', { exact: true })
  if ((await rawSummaryResponse.count()) > 0) {
    await rawSummaryResponse.click()
  }
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
  await page.getByTestId('audit-body-response').locator(':scope > summary').click()
  // Structured view shows merged content in a <pre>; raw SSE line also contains this substring — use exact match to avoid strict-mode duplicate.
  await expect(
    page.getByText('e2e stream chunk', { exact: true }),
  ).toBeVisible({ timeout: 20_000 })
  const responseSection = page.getByTestId('audit-body-response')
  const rawSummaryResponse = responseSection.getByText('原始正文', { exact: true })
  if ((await rawSummaryResponse.count()) > 0) {
    await rawSummaryResponse.click()
  }
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
  await page.getByRole('textbox', { name: '模型' }).fill(model)
  await page.getByRole('button', { name: '查询' }).click()
  await expect(page).toHaveURL(new RegExp(`[?&]model=${encodeURIComponent(model)}`))
})

test('model picker dropdown lists model by recency after query', async ({ page }) => {
  await page.goto('/logs')
  const model = `e2e_recent_${Date.now()}`
  await page.getByRole('textbox', { name: '模型' }).fill(model)
  await page.getByRole('button', { name: '查询' }).click()
  await page
    .getByRole('button', { name: '展开模型列表（OpenRouter 全目录 + 最近使用排序）' })
    .click()
  await expect(
    page.locator('[data-slot="popover-content"]').getByText(model, { exact: true }),
  ).toBeVisible()
})

test('keyword draft does not filter list until query is applied', async ({
  page,
}) => {
  const model = `e2e_draft_kw_${Date.now()}`
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
  const dataRow = page.getByRole('row').filter({ hasText: model })
  await expect(dataRow).toBeVisible({ timeout: 20_000 })

  await page.getByLabel('关键词').fill('e2e_will_not_match_any_request_id_xyz')
  await expect(dataRow).toBeVisible({ timeout: 5_000 })

  await page.getByRole('button', { name: '查询' }).click()
  await expect(page).toHaveURL(/keyword=e2e_will_not_match/)
  await expect(dataRow).not.toBeVisible()
  await expect(page.getByText('暂无日志')).toBeVisible()
})

test('keyword submit via Enter updates URL', async ({ page }) => {
  const model = `e2e_enter_kw_${Date.now()}`
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

  await page.goto('/logs')
  await expect(page.getByRole('row').filter({ hasText: model })).toBeVisible({
    timeout: 20_000,
  })
  await page.getByLabel('关键词').fill(kw)
  await page.getByLabel('关键词').press('Enter')
  await expect(page).toHaveURL(new RegExp(`[?&]keyword=${encodeURIComponent(kw)}`))
  await expect(
    page.getByRole('row').filter({ hasText: row!.request_id }),
  ).toBeVisible({ timeout: 20_000 })
})

test('applied model chip removes model from URL', async ({ page }) => {
  await page.goto('/logs')
  const model = `e2e_chip_${Date.now()}`
  await page.getByRole('textbox', { name: '模型' }).fill(model)
  await page.getByRole('button', { name: '查询' }).click()
  await expect(page).toHaveURL(new RegExp(`[?&]model=${encodeURIComponent(model)}`))

  const chip = page.getByRole('button', { name: new RegExp(`模型：${model}`) })
  await expect(chip).toBeVisible()
  await chip.click()
  await expect(page).not.toHaveURL(new RegExp(`[?&]model=${encodeURIComponent(model)}`))
})

test('logs URL with app_id expands advanced filters', async ({ page }) => {
  const appId = `e2e_adv_open_${Date.now()}`
  const end = unixNow() + 3600
  await page.goto(
    `/logs?app_id=${encodeURIComponent(appId)}&start_time=0&end_time=${end}&offset=0`,
  )
  await expect(page.getByRole('button', { name: /更多条件/ })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  await expect(page.getByLabel('应用 (app_id)')).toBeVisible()
})

test('logs URL with thread_id expands advanced filters', async ({ page }) => {
  const threadId = `e2e_thr_open_${Date.now()}`
  const end = unixNow() + 3600
  await page.goto(
    `/logs?thread_id=${encodeURIComponent(threadId)}&start_time=0&end_time=${end}&offset=0`,
  )
  await expect(page.getByRole('button', { name: /更多条件/ })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  await expect(page.getByLabel('会话 (thread_id)')).toBeVisible()
})

test('HTTP status preset applies status_code to URL', async ({ page }) => {
  await page.goto('/logs')
  await page.getByRole('button', { name: /更多条件/ }).click()
  const filterForm = page
    .locator('form')
    .filter({ has: page.getByRole('button', { name: '查询' }) })
  await filterForm.getByRole('button', { name: '200', exact: true }).click()
  await expect(page).toHaveURL(/[?&]status_code=200(?:&|$)/)
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
