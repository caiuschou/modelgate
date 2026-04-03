import { mkdirSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from '@playwright/test'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const authFile = path.join(__dirname, '../playwright/.auth/user.json')

export default async function globalSetup() {
  const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:3000'
  const invite = process.env.E2E_INVITE_CODE ?? 'e2e-invite-code'
  const username = process.env.E2E_USERNAME ?? 'e2e_user'
  const password = process.env.E2E_PASSWORD ?? 'E2e_local_pass_1'

  mkdirSync(path.dirname(authFile), { recursive: true })

  const reg = await fetch(`${baseURL}/api/v1/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password, invite_code: invite }),
  })
  const regText = await reg.text()
  if (reg.status !== 201 && reg.status !== 409) {
    const wrongInvite =
      reg.status === 400 && regText.includes('Invalid invite code')
    if (!wrongInvite) {
      throw new Error(`globalSetup register failed: ${reg.status} ${regText}`)
    }
  }

  const browser = await chromium.launch()
  const page = await browser.newPage()
  await page.goto(`${baseURL}/login`)
  await page.getByLabel('用户名').fill(username)
  await page.getByLabel('密码').fill(password)
  await page.getByRole('button', { name: '登录' }).click()
  await page.getByRole('heading', { name: '仪表盘' }).waitFor({ timeout: 30_000 })
  await page.context().storageState({ path: authFile })
  await browser.close()
}
