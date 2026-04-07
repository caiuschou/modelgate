import { expect, test } from '@playwright/test'
import {
  createTeam,
  listMyApiKeys,
  listTeamMembers,
  loginApiKey,
  registerTeamMemberOnBehalf,
} from './helpers/api'
import { loadE2eSessionCredentials } from './load-e2e-credentials'

const consoleBase = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:3000'

test.describe('团队与工作空间', () => {
  test('侧栏可进入团队页', async ({ page }) => {
    await page.goto('/')
    await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
    await page.getByRole('link', { name: '团队' }).click()
    await expect(page).toHaveURL(/\/teams$/)
    await expect(page.getByRole('heading', { name: '团队' })).toBeVisible()
  })

  test('切换工作空间后 API 密钥页显示对应上下文', async ({ page }) => {
    const { username, password } = loadE2eSessionCredentials()
    const token = await loginApiKey(consoleBase, username, password)
    const slug = `e2e-${Date.now()}`
    const { team } = await createTeam(consoleBase, token, {
      name: `E2E 团队 ${slug.slice(-6)}`,
      slug,
    })

    const inTeam = await listMyApiKeys(consoleBase, token, {
      teamId: team.id,
    })
    expect(Array.isArray(inTeam)).toBeTruthy()

    await page.goto('/')
    await expect(page.getByRole('heading', { name: '仪表盘' })).toBeVisible()
    await page.getByLabel('工作空间').selectOption({ value: String(team.id) })
    await page.getByRole('link', { name: 'API 密钥' }).click()
    await expect(page).toHaveURL(/\/api-keys$/)
    await expect(
      page
        .locator('p')
        .filter({ hasText: '当前工作空间：' })
        .filter({ hasText: team.name }),
    ).toBeVisible()

    await page.getByLabel('工作空间').selectOption({ label: '个人空间' })
    await expect(
      page
        .locator('p')
        .filter({ hasText: '当前工作空间：' })
        .filter({ hasText: '个人空间' }),
    ).toBeVisible()
  })

  test('管理员可代为注册新成员并加入团队', async ({ page }) => {
    const { username, password } = loadE2eSessionCredentials()
    const token = await loginApiKey(consoleBase, username, password)
    const slug = `reg-${Date.now()}`
    const { team } = await createTeam(consoleBase, token, {
      name: `代为注册团队 ${slug.slice(-6)}`,
      slug,
    })
    const newUsername = `onbehalf_${Date.now()}`
    const initialPass = 'E2e_init_9_char'
    await registerTeamMemberOnBehalf(consoleBase, token, team.id, {
      username: newUsername,
      password: initialPass,
      role: 'member',
    })
    const members = await listTeamMembers(consoleBase, token, team.id)
    expect(members.some((m) => m.username === newUsername)).toBeTruthy()

    const newbieToken = await loginApiKey(consoleBase, newUsername, initialPass)
    expect(newbieToken.length).toBeGreaterThan(20)

    await page.goto(`/teams/${team.id}/members`)
    await expect(
      page.getByRole('row').filter({ hasText: newUsername }),
    ).toBeVisible({ timeout: 15_000 })
  })
})
