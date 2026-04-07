import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const sessionFile = path.join(__dirname, '../playwright/.auth/e2e-session.json')

/**
 * Credentials for the user created in `global-setup.ts` (random username unless `E2E_USERNAME` is set).
 */
export function loadE2eSessionCredentials(): {
  username: string
  password: string
} {
  if (existsSync(sessionFile)) {
    return JSON.parse(readFileSync(sessionFile, 'utf8')) as {
      username: string
      password: string
    }
  }
  return {
    username: process.env.E2E_USERNAME ?? 'e2e_user',
    password: process.env.E2E_PASSWORD ?? 'E2e_local_pass_1',
  }
}
