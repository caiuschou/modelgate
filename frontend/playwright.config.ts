import { defineConfig, devices } from '@playwright/test'

/** Random ports in a wide private range; avoid duplicates. Env vars pin a port when set. */
function allocatePorts(): {
  gatewayPort: number
  vitePort: number
  mockPort: number
} {
  const used = new Set<number>()

  function allocPort(envName: string): number {
    const fromEnv = Number(process.env[envName])
    if (Number.isFinite(fromEnv) && fromEnv > 0 && fromEnv < 65536) {
      if (used.has(fromEnv)) {
        throw new Error(`E2E: duplicate port ${fromEnv} from ${envName}`)
      }
      used.add(fromEnv)
      return fromEnv
    }
    for (let i = 0; i < 500; i++) {
      const p = 35000 + Math.floor(Math.random() * 25000)
      if (!used.has(p)) {
        used.add(p)
        return p
      }
    }
    throw new Error('E2E: could not allocate free port')
  }

  return {
    gatewayPort: allocPort('E2E_BACKEND_PORT'),
    vitePort: allocPort('E2E_FRONTEND_PORT'),
    mockPort: allocPort('E2E_MOCK_PORT'),
  }
}

const { gatewayPort, vitePort, mockPort } = allocatePorts()

const gatewayUrl = `http://127.0.0.1:${gatewayPort}`
const baseURL = `http://127.0.0.1:${vitePort}`
const mockV1 = `http://127.0.0.1:${mockPort}/v1`

process.env.E2E_BACKEND_URL = gatewayUrl
process.env.PLAYWRIGHT_BASE_URL = baseURL
process.env.E2E_GATEWAY_URL = gatewayUrl

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'list',
  globalSetup: './e2e/global-setup.ts',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  use: {
    baseURL,
    storageState: 'playwright/.auth/user.json',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    ...devices['Desktop Chrome'],
  },
  webServer: [
    {
      command: 'node ../e2e/run-modelgate-stack.mjs',
      url: `${gatewayUrl}/healthz`,
      reuseExistingServer: !process.env.CI,
      timeout: 180_000,
      env: {
        ...process.env,
        MODELGATE_SERVER_PORT: String(gatewayPort),
        E2E_MOCK_UPSTREAM_PORT: String(mockPort),
        UPSTREAM_BASE_URL: mockV1,
      },
    },
    {
      command: `npm run dev -- --host 127.0.0.1 --port ${vitePort} --strictPort`,
      url: baseURL,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
      env: {
        ...process.env,
        E2E_GATEWAY_URL: gatewayUrl,
      },
    },
  ],
})
