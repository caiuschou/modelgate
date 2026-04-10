import { describe, expect, it } from 'vitest'
import {
  auditBodyPhysicalLine,
  auditBodyRequestSemanticLine,
  auditBodyResponseSemanticLine,
  countSseDataEvents,
} from '@/features/logs/audit-body-summary'
import type { ParsedChatCompletionResponse } from '@/features/logs/parse-chat-completion-display'
import {
  parseChatCompletionRequestBody,
  parseChatCompletionResponseBody,
} from '@/features/logs/parse-chat-completion-display'

describe('auditBodyPhysicalLine', () => {
  it('formats bytes and line count', () => {
    expect(auditBodyPhysicalLine('')).toMatch(/0 B · 0 行/)
    expect(auditBodyPhysicalLine('a\nb')).toMatch(/2 行/)
  })

  it('uses MB for very large UTF-8 bodies', () => {
    const raw = 'a'.repeat(1024 * 1024)
    expect(auditBodyPhysicalLine(raw)).toMatch(/MB ·/)
  })
})

describe('countSseDataEvents', () => {
  it('counts data lines excluding DONE', () => {
    const raw = [
      'data: {"choices":[]}',
      '',
      'data: [DONE]',
      'data: {"x":1}',
    ].join('\n')
    expect(countSseDataEvents(raw)).toBe(2)
  })
})

describe('auditBodyRequestSemanticLine', () => {
  it('summarizes messages, stream, and tools', () => {
    const raw = JSON.stringify({
      messages: [{ role: 'user', content: 'hi' }],
      stream: true,
      tools: [{ type: 'function', function: { name: 'x' } }],
    })
    const parsed = parseChatCompletionRequestBody(raw)
    expect(auditBodyRequestSemanticLine(raw, parsed)).toBe(
      '1 条消息 · 流式 · 1 个工具',
    )
  })

  it('falls back to raw messages length when parsed is null', () => {
    const raw = JSON.stringify({
      messages: [{ role: 'user', content: 'x' }],
    })
    expect(auditBodyRequestSemanticLine(raw, null)).toBe('1 条消息')
  })
})

describe('auditBodyResponseSemanticLine', () => {
  it('summarizes JSON choices', () => {
    const raw = JSON.stringify({
      choices: [{ message: { role: 'assistant', content: 'ok' } }],
    })
    const parsed = parseChatCompletionResponseBody(raw)
    expect(auditBodyResponseSemanticLine(raw, parsed)).toBe('1 个 choice')
  })

  it('includes tool calls on JSON completion responses', () => {
    const raw = JSON.stringify({
      choices: [
        {
          message: {
            role: 'assistant',
            content: '',
            tool_calls: [
              {
                id: 'c1',
                type: 'function',
                function: { name: 'fn', arguments: '{}' },
              },
            ],
          },
        },
      ],
    })
    const parsed = parseChatCompletionResponseBody(raw)
    expect(auditBodyResponseSemanticLine(raw, parsed)).toBe(
      '1 个 choice · 1 次工具调用',
    )
  })

  it('falls back to tool-call-only summary when shape is not JSON choices or SSE', () => {
    const parsed: ParsedChatCompletionResponse = {
      content: '',
      reasoning: '',
      refusal: '',
      tool_calls: [{ id: '1', name: 't', arguments: '{}' }],
      source: 'json',
    }
    expect(auditBodyResponseSemanticLine('{}', parsed)).toBe('1 次工具调用')
  })

  it('returns null when there is nothing to summarize', () => {
    expect(auditBodyResponseSemanticLine('{}', null)).toBeNull()
  })

  it('summarizes SSE and tool calls', () => {
    const raw = [
      'data: {"choices":[{"delta":{"content":"a"}}]}',
      'data: {"choices":[{"delta":{"tool_calls":[{"id":"1","function":{"name":"f","arguments":""}}]}}]}',
    ].join('\n')
    const parsed = parseChatCompletionResponseBody(raw)
    expect(auditBodyResponseSemanticLine(raw, parsed)).toContain('条 SSE')
    expect(auditBodyResponseSemanticLine(raw, parsed)).toContain('工具调用')
  })
})
