import { expect, test } from '@playwright/test'
import { changeMyPassword, loginApiKey } from './helpers/api'

const consoleBase = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:5173'

test.use({ storageState: { cookies: [], origins: [] } })

test('change password via UI then login with new password', async ({
  page,
  request,
}) => {
  const invite = process.env.E2E_INVITE_CODE ?? 'e2e-invite-code'
  const username = `e2e_chpw_${Date.now()}`
  const oldPass = 'E2e_old_pass_1'
  const newPass = 'E2e_new_pass_2'

  const reg = await request.post('/api/v1/auth/register', {
    data: { username, password: oldPass, invite_code: invite },
  })
  expect(reg.status(), await reg.text()).toBe(201)

  await page.goto('/login')
  await page.getByLabel('用户名').fill(username)
  await page.getByLabel('密码').fill(oldPass)
  await Promise.all([
    page.waitForURL((url) => url.pathname === '/', { timeout: 20_000 }),
    page.getByRole('button', { name: '登录' }).click(),
  ])

  await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
  await page.getByRole('button', { name: `账号菜单：${username}` }).click()
  await page.getByRole('menuitem', { name: '修改密码' }).click()
  const dialog = page.getByRole('dialog', { name: '修改密码' })
  await expect(dialog).toBeVisible()
  await dialog.getByLabel('新密码', { exact: true }).fill(newPass)
  await dialog.getByLabel('确认新密码').fill(newPass)
  await dialog.getByRole('button', { name: '保存新密码' }).click()

  await expect(page).toHaveURL(
    new RegExp(
      `/login\\?username=${encodeURIComponent(username)}&password_changed=1`,
    ),
  )
  await expect(
    page.getByText('密码已更新，请使用新密码登录。'),
  ).toBeVisible()

  await page.getByLabel('密码').fill(newPass)
  await Promise.all([
    page.waitForURL((url) => url.pathname === '/', { timeout: 20_000 }),
    page.getByRole('button', { name: '登录' }).click(),
  ])
  await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
})

test('change password API returns 204 with valid session', async ({ request }) => {
  const invite = process.env.E2E_INVITE_CODE ?? 'e2e-invite-code'
  const username = `e2e_chpw_api_${Date.now()}`
  const password = 'E2e_wrong_api_1'

  const reg = await request.post('/api/v1/auth/register', {
    data: { username, password, invite_code: invite },
  })
  expect(reg.status(), await reg.text()).toBe(201)

  const token = await loginApiKey(consoleBase, username, password)
  const r = await changeMyPassword(consoleBase, token, {
    new_password: 'E2e_new99_x',
  })
  expect(r.status, await r.text()).toBe(204)
})
