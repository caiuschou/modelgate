/** E2E helpers hitting the real Rust API (see `e2e/run-modelgate-stack.mjs`). */

/** Returns a console session JWT (Bearer for `/api/v1/*`). */
export async function loginApiKey(
  consoleBaseUrl: string,
  username: string,
  password: string,
): Promise<string> {
  const r = await fetch(`${consoleBaseUrl}/api/v1/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  })
  if (!r.ok) {
    throw new Error(`login failed: ${r.status} ${await r.text()}`)
  }
  const body = (await r.json()) as { token: string }
  return body.token
}

/** Chat completions require `sk-or-v1-*`; create a disposable gateway key using a session JWT. */
export async function getGatewayApiKeyForSession(
  consoleBaseUrl: string,
  sessionToken: string,
): Promise<string> {
  const { api_key } = await createMyApiKey(consoleBaseUrl, sessionToken, {
    name: `e2e-gw-${Date.now()}`,
  })
  return api_key
}

export async function createChatCompletion(
  backendBaseUrl: string,
  apiKey: string,
  model: string,
  options?: { appId?: string },
): Promise<Response> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${apiKey}`,
  }
  if (options?.appId) {
    headers['X-App-Id'] = options.appId
  }
  return fetch(`${backendBaseUrl}/v1/chat/completions`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      model,
      messages: [{ role: 'user', content: 'e2e audit ping' }],
      stream: false,
    }),
  })
}

export type ApiKeySummary = {
  id: number
  name: string
  description: string
  preview: string
  created_at: number
  last_used_at: number | null
  revoked: boolean
  disabled: boolean
  expires_at: number | null
  quota_monthly_tokens: number | null
  quota_used_tokens: number
  model_allowlist: string[] | null
  ip_allowlist: string[] | null
  status: string
}

export async function listMyApiKeys(
  consoleBaseUrl: string,
  token: string,
): Promise<ApiKeySummary[]> {
  const r = await fetch(`${consoleBaseUrl}/api/v1/me/api-keys`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  if (!r.ok) {
    throw new Error(`list me/api-keys failed: ${r.status} ${await r.text()}`)
  }
  const body = (await r.json()) as { data: ApiKeySummary[] }
  return body.data ?? []
}

export async function createMyApiKey(
  consoleBaseUrl: string,
  token: string,
  body?: Record<string, unknown>,
): Promise<{ id: number; api_key: string; created_at: number }> {
  const r = await fetch(`${consoleBaseUrl}/api/v1/me/api-keys`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body ?? { name: 'e2e-key' }),
  })
  if (!r.ok) {
    throw new Error(`create me/api-keys failed: ${r.status} ${await r.text()}`)
  }
  return r.json() as Promise<{ id: number; api_key: string; created_at: number }>
}

export async function revokeMyApiKey(
  consoleBaseUrl: string,
  token: string,
  keyId: number,
): Promise<void> {
  const r = await fetch(
    `${consoleBaseUrl}/api/v1/me/api-keys/${keyId}/revoke`,
    {
      method: 'POST',
      headers: { Authorization: `Bearer ${token}` },
    },
  )
  if (!r.ok) {
    throw new Error(`revoke me/api-keys failed: ${r.status} ${await r.text()}`)
  }
}

export async function waitForAuditListRow(
  backendBaseUrl: string,
  apiKey: string,
  query: Record<string, string>,
  timeoutMs = 25_000,
): Promise<{ request_id: string } | null> {
  const qs = new URLSearchParams(query)
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const r = await fetch(`${backendBaseUrl}/api/v1/logs/request?${qs}`, {
      headers: { Authorization: `Bearer ${apiKey}` },
    })
    if (r.ok) {
      const body = (await r.json()) as {
        data: { request_id: string }[]
        total: number
      }
      if (body.data?.length) {
        return { request_id: body.data[0].request_id }
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 500))
  }
  return null
}
