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

export async function changeMyPassword(
  consoleBaseUrl: string,
  sessionToken: string,
  body: { new_password: string },
): Promise<Response> {
  return fetch(`${consoleBaseUrl}/api/v1/me/password`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify(body),
  })
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

/** Drain the body; streaming audit finalizes after the upstream closes the body. */
export async function createChatCompletionStream(
  backendBaseUrl: string,
  apiKey: string,
  model: string,
): Promise<Response> {
  return fetch(`${backendBaseUrl}/v1/chat/completions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model,
      messages: [{ role: 'user', content: 'e2e stream ping' }],
      stream: true,
    }),
  })
}

export async function createChatCompletion(
  backendBaseUrl: string,
  apiKey: string,
  model: string,
  options?: { appId?: string; threadId?: string },
): Promise<Response> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${apiKey}`,
  }
  if (options?.appId) {
    headers['X-App-Id'] = options.appId
  }
  if (options?.threadId) {
    headers['X-Thread-Id'] = options.threadId
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
  max_concurrent_requests?: number | null
  quota_monthly_spend_minor?: string | null
  quota_used_spend_minor?: string
  model_allowlist: string[] | null
  ip_allowlist: string[] | null
  status: string
  team_id?: number | null
  default_byok_profile_id?: number | null
}

export async function listMyApiKeys(
  consoleBaseUrl: string,
  token: string,
  options?: { teamId?: number | null },
): Promise<ApiKeySummary[]> {
  const headers: Record<string, string> = { Authorization: `Bearer ${token}` }
  if (options?.teamId != null) {
    headers['X-Team-Id'] = String(options.teamId)
  }
  const r = await fetch(`${consoleBaseUrl}/api/v1/me/api-keys`, {
    headers,
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
  options?: { teamId?: number | null },
): Promise<{ id: number; api_key: string; created_at: number }> {
  const headers: Record<string, string> = {
    Authorization: `Bearer ${token}`,
    'Content-Type': 'application/json',
  }
  if (options?.teamId != null) {
    headers['X-Team-Id'] = String(options.teamId)
  }
  const r = await fetch(`${consoleBaseUrl}/api/v1/me/api-keys`, {
    method: 'POST',
    headers,
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

/** Poll detail until `response_body_path` is set (streaming second-phase update). */
export async function waitForAuditDetailResponsePath(
  backendBaseUrl: string,
  token: string,
  requestId: string,
  timeoutMs = 25_000,
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const r = await fetch(
      `${backendBaseUrl}/api/v1/logs/request/${encodeURIComponent(requestId)}`,
      { headers: { Authorization: `Bearer ${token}` } },
    )
    if (r.ok) {
      const row = (await r.json()) as { response_body_path?: string | null }
      if (row.response_body_path) {
        return true
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 300))
  }
  return false
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

/** Poll `GET /api/v1/logs/threads` until a row appears (session-centric list). */
export async function waitForAuditThreadListRow(
  backendBaseUrl: string,
  token: string,
  query: Record<string, string>,
  timeoutMs = 25_000,
): Promise<{
  thread_id: string
  request_count: number
  last_prompt_preview?: string | null
} | null> {
  const qs = new URLSearchParams(query)
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const r = await fetch(`${backendBaseUrl}/api/v1/logs/threads?${qs}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    if (r.ok) {
      const body = (await r.json()) as {
        data: {
          thread_id: string
          request_count: number
          last_prompt_preview?: string | null
        }[]
      }
      if (body.data?.length) {
        return {
          thread_id: body.data[0].thread_id,
          request_count: body.data[0].request_count,
          last_prompt_preview: body.data[0].last_prompt_preview,
        }
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 500))
  }
  return null
}

export async function createTeam(
  consoleBaseUrl: string,
  token: string,
  body: { name: string; slug: string },
): Promise<{ team: { id: number; name: string; slug: string; role: string } }> {
  const r = await fetch(`${consoleBaseUrl}/api/v1/teams`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  })
  if (!r.ok) {
    throw new Error(`create team failed: ${r.status} ${await r.text()}`)
  }
  return (await r.json()) as {
    team: { id: number; name: string; slug: string; role: string }
  }
}

export async function listTeamMembers(
  consoleBaseUrl: string,
  token: string,
  teamId: number,
): Promise<{ user_id: number; username: string; role: string; joined_at: number }[]> {
  const r = await fetch(`${consoleBaseUrl}/api/v1/teams/${teamId}/members`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  if (!r.ok) {
    throw new Error(`list team members failed: ${r.status} ${await r.text()}`)
  }
  const body = (await r.json()) as {
    data: { user_id: number; username: string; role: string; joined_at: number }[]
  }
  return body.data ?? []
}

export async function registerTeamMemberOnBehalf(
  consoleBaseUrl: string,
  token: string,
  teamId: number,
  body: { username: string; password: string; role: string },
): Promise<{ user_id: number; username: string }> {
  const r = await fetch(
    `${consoleBaseUrl}/api/v1/teams/${teamId}/members/register-on-behalf`,
    {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    },
  )
  if (!r.ok) {
    throw new Error(
      `register member on behalf failed: ${r.status} ${await r.text()}`,
    )
  }
  return r.json() as Promise<{ user_id: number; username: string }>
}
