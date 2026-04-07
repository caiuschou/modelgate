import { mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import type { FullConfig } from '@playwright/test'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const authDir = path.join(__dirname, '../playwright/.auth')
const authFile = path.join(authDir, 'user.json')
const sessionCredFile = path.join(authDir, 'e2e-session.json')

type LoginJson = {
  token: string
  user: { username: string; role: string }
}

function resolveBaseURL(config: FullConfig): string {
  for (const p of config.projects ?? []) {
    const u = p.use?.baseURL
    if (typeof u === 'string' && u.length > 0) {
      return u
    }
  }
  const fromEnv = process.env.PLAYWRIGHT_BASE_URL
  if (typeof fromEnv === 'string' && fromEnv.length > 0) {
    return fromEnv
  }
  return 'http://127.0.0.1:3000'
}

export default async function globalSetup(config: FullConfig) {
  mkdirSync(authDir, { recursive: true })
  const baseURL = resolveBaseURL(config)
  process.env.PLAYWRIGHT_BASE_URL = baseURL
  const invite = process.env.E2E_INVITE_CODE ?? 'e2e-invite-code'
  // Unique default user so a reused `e2e/modelgate-e2e.db` cannot 409 with a mismatched password.
  const username =
    process.env.E2E_USERNAME ??
    `e2e_user_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
  const password = process.env.E2E_PASSWORD ?? 'E2e_local_pass_1'

  const reg = await fetch(`${baseURL}/api/v1/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password, invite_code: invite }),
  })
  const regText = await reg.text()
  if (reg.status === 201) {
    // created
  } else if (reg.status === 409 && process.env.E2E_USERNAME) {
    // caller-selected user may already exist in a reused DB
  } else {
    throw new Error(`globalSetup register failed: ${reg.status} ${regText}`)
  }

  const loginRes = await fetch(`${baseURL}/api/v1/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  })
  const loginText = await loginRes.text()
  if (!loginRes.ok) {
    throw new Error(`globalSetup login failed: ${loginRes.status} ${loginText}`)
  }
  const data = JSON.parse(loginText) as LoginJson
  const role = data.user.role === 'admin' ? 'admin' : 'user'

  const bearerProbe = await fetch(`${baseURL}/api/v1/teams`, {
    headers: { Authorization: `Bearer ${data.token}` },
  })
  if (!bearerProbe.ok) {
    const body = await bearerProbe.text()
    throw new Error(
      `globalSetup GET /api/v1/teams with session JWT failed: ${bearerProbe.status} ${body}`,
    )
  }

  writeFileSync(
    sessionCredFile,
    JSON.stringify({ username, password }, null, 2),
    'utf8',
  )

  // Match `zustand` persist (`auth-store.ts` name `modelgate-auth`) — avoid brittle UI form typing.
  const origin = new URL(baseURL).origin
  const persisted = {
    state: {
      token: data.token,
      user: {
        username: data.user.username,
        role,
      },
    },
    version: 0,
  }

  const teamPersisted = {
    state: {
      currentTeamId: null,
    },
    version: 0,
  }

  writeFileSync(
    authFile,
    JSON.stringify(
      {
        cookies: [],
        origins: [
          {
            origin,
            localStorage: [
              {
                name: 'modelgate-auth',
                value: JSON.stringify(persisted),
              },
              {
                name: 'modelgate-team',
                value: JSON.stringify(teamPersisted),
              },
            ],
          },
        ],
      },
      null,
      2,
    ),
    'utf8',
  )
}
