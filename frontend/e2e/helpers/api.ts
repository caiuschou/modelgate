/** E2E helpers hitting the real Rust API (see `e2e/run-modelgate-stack.mjs`). */

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
