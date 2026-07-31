import { expect, test, type Page } from '@playwright/test'
import {
  createChatCompletion,
  createMyApiKey,
  listMyApiKeys,
  loginApiKey,
  patchMyApiKey,
  revokeMyApiKey,
} from './helpers/api'
import { consoleSidebarNav } from './helpers/console-nav'
import { loadE2eSessionCredentials } from './load-e2e-credentials'

const consoleBase = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:5173'
const backendBase = process.env.E2E_BACKEND_URL ?? 'http://127.0.0.1:8000'

/** Mock upstream model id (see `e2e/mock-openai-upstream.mjs`). */
const MOCK_MODEL = 'e2e-mock-model'

let e2eUser: string
let e2ePass: string

test.beforeAll(() => {
  const c = loadE2eSessionCredentials()
  e2eUser = c.username
  e2ePass = c.password
})

async function gotoApiKeys(page: Page) {
  await page.goto('/dashboard')
  await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
  await consoleSidebarNav(page).getByRole('link', { name: 'API 密钥' }).click()
  await expect(page).toHaveURL(/\/api-keys$/)
  await expect(page.getByRole('heading', { name: 'API 密钥' })).toBeVisible()
  await expect(async () => {
    if (await page.getByText('加载中…').isVisible()) return false
    const tableRows = await page.locator('table tbody tr').count()
    const empty = await page.getByText('暂无密钥', { exact: false }).isVisible()
    const failed = await page.getByText('加载失败').isVisible()
    return failed || tableRows > 0 || empty
  }).toPass({ timeout: 20_000 })
}

function expectChatCompletionOk(res: Response, bodyText: string) {
  expect(res.ok, bodyText).toBe(true)
  const j = JSON.parse(bodyText) as { choices?: unknown[] }
  expect(
    j.choices?.length,
    'OpenAI-style response should include choices',
  ).toBeGreaterThan(0)
}

test('session affinity: X-Thread-Id + platform pool → two chats succeed', async () => {
  const token = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const { id, api_key } = await createMyApiKey(consoleBase, token, {
    name: `e2e-session-aff-${Date.now()}`,
  })
  try {
    const patch = await patchMyApiKey(consoleBase, token, id, {
      session_affinity_enabled: true,
      upstream_pool: [{ kind: 'platform' }],
    })
    expect(patch.ok, await patch.text()).toBe(true)

    const threadId = `e2e_thread_${Date.now()}`
    const r1 = await createChatCompletion(backendBase, api_key, MOCK_MODEL, {
      threadId,
    })
    const t1 = await r1.text()
    expectChatCompletionOk(r1, t1)

    const r2 = await createChatCompletion(backendBase, api_key, MOCK_MODEL, {
      threadId,
    })
    const t2 = await r2.text()
    expectChatCompletionOk(r2, t2)
  } finally {
    await revokeMyApiKey(consoleBase, token, id)
  }
})

test('session affinity: body user + platform pool → two chats succeed', async () => {
  const token = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const { id, api_key } = await createMyApiKey(consoleBase, token, {
    name: `e2e-session-user-${Date.now()}`,
  })
  try {
    const patch = await patchMyApiKey(consoleBase, token, id, {
      session_affinity_enabled: true,
      upstream_pool: [{ kind: 'platform' }],
    })
    expect(patch.ok, await patch.text()).toBe(true)

    const bodyUser = `e2e_body_user_${Date.now()}`
    const r1 = await createChatCompletion(backendBase, api_key, MOCK_MODEL, {
      user: bodyUser,
    })
    const t1 = await r1.text()
    expectChatCompletionOk(r1, t1)

    const r2 = await createChatCompletion(backendBase, api_key, MOCK_MODEL, {
      user: bodyUser,
    })
    const t2 = await r2.text()
    expectChatCompletionOk(r2, t2)
  } finally {
    await revokeMyApiKey(consoleBase, token, id)
  }
})

test('API key detail shows session upstream section', async ({ page }) => {
  const token = await loginApiKey(consoleBase, e2eUser, e2ePass)
  const keys = await listMyApiKeys(consoleBase, token)
  const activeKey = keys.find((k) => !k.revoked)
  expect(
    activeKey,
    'need at least one non-revoked API key (global setup registers user)',
  ).toBeTruthy()

  await gotoApiKeys(page)
  await page.getByRole('link', { name: activeKey!.name }).first().click()
  await expect(page).toHaveURL(new RegExp(`/api-keys/${activeKey!.id}$`))
  await expect(
    page.getByRole('heading', { name: '会话上游（亲和）' }),
  ).toBeVisible()
  await expect(
    page.getByRole('button', { name: '保存会话上游' }),
  ).toBeVisible()
})
