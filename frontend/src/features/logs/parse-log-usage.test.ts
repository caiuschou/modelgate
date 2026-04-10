import { describe, expect, it } from 'vitest'
import { parseUsageFromBody } from '@/features/logs/parse-log-usage'

describe('parseUsageFromBody', () => {
  it('reads usage from JSON object', () => {
    const raw = JSON.stringify({
      usage: {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
        prompt_tokens_details: { cached_tokens: 5 },
      },
    })
    const u = parseUsageFromBody(raw)
    expect(u?.prompt_tokens).toBe(10)
    expect(u?.prompt_tokens_details?.cached_tokens).toBe(5)
  })

  it('reads usage from last SSE data line', () => {
    const raw = [
      'data: {"choices":[{"delta":{"content":"x"}}]}',
      '',
      'data: {"usage":{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}',
    ].join('\n')
    const u = parseUsageFromBody(raw)
    expect(u?.total_tokens).toBe(3)
  })
})
