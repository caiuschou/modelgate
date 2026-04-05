import { expect, test, type Page } from '@playwright/test'
import {
  createChatCompletion,
  createMyApiKey,
  listMyApiKeys,
  loginApiKey,
  revokeMyApiKey,
} from './helpers/api'

const consoleBase = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:3000'
const backendBase = process.env.E2E_BACKEND_URL ?? 'http://127.0.0.1:8000'
const e2eUser = process.env.E2E_USERNAME ?? 'e2e_user'
const e2ePass = process.env.E2E_PASSWORD ?? 'E2e_local_pass_1'

/** 与侧栏用例一致：先加载 `/` 再客户端跳转，避免直链 `/api-keys` 时 persist 未恢复 */
async function gotoApiKeys(page: Page) {
  await page.goto('/')
  await page.locator('aside a[href="/api-keys"]').click()
  await expect(page).toHaveURL(/\/api-keys$/)
}

test.describe('API 密钥页（已登录）', () => {
  test.describe.configure({ mode: 'serial' })

  test('侧栏可进入且展示标题与主操作', async ({ page }) => {
    await page.goto('/')
    await page.locator('aside a[href="/api-keys"]').click()
    await expect(page).toHaveURL(/\/api-keys$/)
    await expect(page.getByRole('heading', { name: 'API 密钥' })).toBeVisible()
    await expect(
      page.getByText('与模型用量中的 Token 计数不同', { exact: false }),
    ).toBeVisible()
    await expect(page.getByRole('button', { name: '新建密钥' })).toBeVisible()
  })

  test('直接访问 /api-keys 可加载列表或空状态', async ({ page }) => {
    await gotoApiKeys(page)
    await expect(page.getByRole('heading', { name: 'API 密钥' })).toBeVisible()
    const hasTable = await page.locator('table').count()
    const empty = page.getByText('暂无密钥', { exact: false })
    expect(hasTable > 0 || (await empty.isVisible())).toBeTruthy()
  })

  test('新建密钥：表单填写名称后生成，仅一次展示完整 sk-or-v1-', async ({
    page,
  }) => {
    const keyName = `e2e-playwright-${Date.now()}`
    const token = await loginApiKey(consoleBase, e2eUser, e2ePass)
    const before = await listMyApiKeys(consoleBase, token)

    await gotoApiKeys(page)
    await page.getByRole('button', { name: '新建密钥' }).click()
    await expect(page.getByPlaceholder('例如：生产-支付助手')).toBeVisible({
      timeout: 10_000,
    })
    await page.getByPlaceholder('例如：生产-支付助手').fill(keyName)
    await page.getByRole('button', { name: '生成密钥' }).click()
    await expect(
      page.getByText('请立即保存 — 完整密钥仅显示这一次', { exact: false }),
    ).toBeVisible({ timeout: 15_000 })

    const secretPre = page.locator('pre').filter({ hasText: /^sk-or-v1-/ })
    await expect(secretPre).toBeVisible()
    const fullKey = (await secretPre.innerText()).trim()
    expect(fullKey.startsWith('sk-or-v1-')).toBeTruthy()
    expect(fullKey.length).toBeGreaterThan(20)

    await page.getByRole('button', { name: '我已保存' }).click()
    await expect(secretPre).toHaveCount(0)

    const after = await listMyApiKeys(consoleBase, token)
    expect(after.length).toBe(before.length + 1)
    const created = after.find((k) => !before.some((b) => b.id === k.id))
    expect(created, '新密钥应出现在 API 列表').toBeTruthy()
    expect(created!.preview).toContain('…')
    expect(created!.preview.length).toBeLessThan(fullKey.length)
    expect(created!.revoked).toBe(false)
    expect(created!.name).toContain('e2e-playwright')

    const row = page.getByRole('row').filter({ hasText: keyName })
    await expect(row).toBeVisible()
    await expect(row.getByText('有效', { exact: true })).toBeVisible()
    await expect(row).not.toContainText(fullKey)
  })

  test('复制完整密钥：写入剪贴板', async ({ page, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write'], {
      origin: consoleBase,
    })

    await gotoApiKeys(page)
    await page.getByRole('button', { name: '新建密钥' }).click()
    await page.getByPlaceholder('例如：生产-支付助手').fill('clipboard-test')
    await page.getByRole('button', { name: '生成密钥' }).click()
    await expect(page.locator('pre').filter({ hasText: /^sk-or-v1-/ })).toBeVisible({
      timeout: 15_000,
    })
    const fullKey = (
      await page.locator('pre').filter({ hasText: /^sk-or-v1-/ }).innerText()
    ).trim()

    await page.getByRole('button', { name: '复制完整密钥' }).click()
    await expect(page.getByText('已复制到剪贴板', { exact: false })).toBeVisible({
      timeout: 5_000,
    })

    const fromClipboard = await page.evaluate(() => navigator.clipboard.readText())
    expect(fromClipboard).toBe(fullKey)
  })

  test('吊销：确认对话框后行变为已吊销，且该密钥无法调用 chat', async ({
    page,
  }) => {
    const token = await loginApiKey(consoleBase, e2eUser, e2ePass)
    const { id, api_key: disposableKey } = await createMyApiKey(consoleBase, token, {
      name: 'e2e-disposable',
    })
    const rowSummary = (await listMyApiKeys(consoleBase, token)).find((k) => k.id === id)!
    expect(rowSummary.preview).toBeTruthy()

    await gotoApiKeys(page)
    const row = page.getByRole('row').filter({ hasText: 'e2e-disposable' })

    page.once('dialog', (d) => {
      expect(d.message()).toContain('吊销')
      expect(d.message()).toContain(rowSummary.preview)
      void d.accept()
    })

    await row.getByRole('button', { name: '吊销' }).click()
    await expect(row.getByText('已吊销', { exact: true })).toBeVisible({
      timeout: 15_000,
    })
    await expect(row.getByRole('button', { name: '吊销' })).toHaveCount(0)

    const chat = await createChatCompletion(backendBase, disposableKey, 'gpt-e2e-revoked')
    expect(chat.status).toBe(401)
  })

  test('列表与后端 GET /api/v1/me/api-keys 条数一致', async ({ page }) => {
    const token = await loginApiKey(consoleBase, e2eUser, e2ePass)
    const keys = await listMyApiKeys(consoleBase, token)

    await gotoApiKeys(page)
    if (keys.length === 0) {
      await expect(page.getByText('暂无密钥', { exact: false })).toBeVisible()
    } else {
      await expect(page.locator('tbody tr')).toHaveCount(keys.length, {
        timeout: 15_000,
      })
    }
  })

  test('详情页与「相关日志」预填 token_id', async ({ page }) => {
    const token = await loginApiKey(consoleBase, e2eUser, e2ePass)
    const keys = await listMyApiKeys(consoleBase, token)
    expect(keys.length).toBeGreaterThan(0)
    const first = keys[0]!

    await gotoApiKeys(page)
    await page.getByRole('link', { name: first.name }).first().click()
    await expect(page).toHaveURL(new RegExp(`/api-keys/${first.id}$`))
    await expect(page.getByRole('heading', { name: first.name })).toBeVisible()

    await page.getByRole('link', { name: '相关日志' }).click()
    await expect(page).toHaveURL(new RegExp(`token_id=${first.id}`))
    await expect(page.getByRole('heading', { name: '日志中心' })).toBeVisible()
    await expect(page.getByPlaceholder('与审计日志中的 token_id 一致')).toHaveValue(
      String(first.id),
    )
  })
})
