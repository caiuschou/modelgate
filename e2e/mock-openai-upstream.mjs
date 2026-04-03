/**
 * Minimal OpenAI-compatible upstream for E2E: replaces OpenRouter / OpenAI during tests.
 * Handles non-stream and stream chat completions on POST .../v1/chat/completions
 */
import http from 'node:http'
import { URL } from 'node:url'

const PORT = Number(process.env.E2E_MOCK_UPSTREAM_PORT ?? '18080')

const nonStreamBody = JSON.stringify({
  id: 'chatcmpl-e2e',
  object: 'chat.completion',
  created: Math.floor(Date.now() / 1000),
  model: 'e2e-mock-model',
  choices: [
    {
      index: 0,
      message: { role: 'assistant', content: 'e2e mock reply' },
      finish_reason: 'stop',
    },
  ],
  usage: {
    prompt_tokens: 3,
    completion_tokens: 5,
    total_tokens: 8,
  },
})

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = []
    req.on('data', (c) => chunks.push(c))
    req.on('end', () => resolve(Buffer.concat(chunks)))
    req.on('error', reject)
  })
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url ?? '/', `http://${req.headers.host}`)

  if (req.method === 'POST' && url.pathname === '/v1/chat/completions') {
    const raw = await readBody(req)
    let stream = false
    try {
      const j = JSON.parse(raw.toString() || '{}')
      stream = Boolean(j.stream)
    } catch {
      /* ignore */
    }

    if (stream) {
      res.writeHead(200, {
        'Content-Type': 'text/event-stream; charset=utf-8',
        'Cache-Control': 'no-cache',
      })
      res.write(
        'data: {"id":"e2e","choices":[{"delta":{"content":"x"}}]}\n\n',
      )
      res.write('data: [DONE]\n\n')
      res.end()
      return
    }

    res.writeHead(200, { 'Content-Type': 'application/json' })
    res.end(nonStreamBody)
    return
  }

  res.writeHead(404, { 'Content-Type': 'text/plain' })
  res.end('not found')
})

server.listen(PORT, '127.0.0.1', () => {
  process.stderr.write(`[e2e mock upstream] http://127.0.0.1:${PORT}/v1/chat/completions\n`)
})
