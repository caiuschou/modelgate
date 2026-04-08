import { expect, test } from '@playwright/test'

test.use({ storageState: { cookies: [], origins: [] } })

test('OpenRouter model catalog is public and lists models', async ({ page }) => {
  await page.route('**/openrouter.ai/api/v1/models**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        data: [
          {
            id: 'e2e/model-a',
            name: 'E2E Model A',
            context_length: 8192,
          },
          {
            id: 'e2e/model-b',
            name: 'E2E Model B',
            context_length: 4096,
          },
        ],
      }),
    })
  })

  await page.goto('/models')
  await expect(page.getByRole('heading', { name: 'Models' })).toBeVisible()
  await expect(page.getByText('e2e/model-a')).toBeVisible()
  await expect(page.getByText('E2E Model A')).toBeVisible()

  await page.getByLabel('搜索模型').fill('model-b')
  await expect(page.getByText('e2e/model-b')).toBeVisible()
  await expect(page.getByText('e2e/model-a')).toHaveCount(0)
})
