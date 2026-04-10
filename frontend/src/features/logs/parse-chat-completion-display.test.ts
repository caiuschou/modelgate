import { describe, expect, it } from 'vitest'
import {
  formatMessageContent,
  parseChatCompletionRequestBody,
  parseChatCompletionResponseBody,
  requestHasStructuredDisplay,
  responseHasStructuredDisplay,
} from '@/features/logs/parse-chat-completion-display'

describe('parseChatCompletionResponseBody', () => {
  it('parses JSON assistant message with reasoning and tool_calls', () => {
    const raw = JSON.stringify({
      choices: [
        {
          message: {
            reasoning_content: 'think step',
            content: 'Done.',
            tool_calls: [
              {
                id: 'call_1',
                type: 'function',
                function: { name: 'get_weather', arguments: '{"city":"NYC"}' },
              },
            ],
          },
        },
      ],
    })
    const p = parseChatCompletionResponseBody(raw)
    expect(p?.source).toBe('json')
    expect(p?.reasoning).toBe('think step')
    expect(p?.content).toBe('Done.')
    expect(p?.tool_calls).toHaveLength(1)
    expect(p?.tool_calls[0].name).toBe('get_weather')
    expect(p?.tool_calls[0].arguments).toBe('{"city":"NYC"}')
  })

  it('merges streamed deltas for content, reasoning, and tool arguments', () => {
    const raw = [
      'data: {"choices":[{"delta":{"reasoning_content":"a"}}]}',
      'data: {"choices":[{"delta":{"reasoning_content":"b"}}]}',
      'data: {"choices":[{"delta":{"content":"Hi"}}]}',
      `data: ${JSON.stringify({
        choices: [
          {
            delta: {
              tool_calls: [
                {
                  index: 0,
                  id: 'x',
                  type: 'function',
                  function: { name: 'fn', arguments: '{"k"' },
                },
              ],
            },
          },
        ],
      })}`,
      `data: ${JSON.stringify({
        choices: [
          {
            delta: {
              tool_calls: [{ index: 0, function: { arguments: ':"v"}' } }],
            },
          },
        ],
      })}`,
      'data: [DONE]',
    ].join('\n')
    const p = parseChatCompletionResponseBody(raw)
    expect(p?.source).toBe('sse_merged')
    expect(p?.reasoning).toBe('ab')
    expect(p?.content).toBe('Hi')
    expect(p?.tool_calls[0].name).toBe('fn')
    expect(p?.tool_calls[0].id).toBe('x')
    expect(p?.tool_calls[0].arguments).toBe('{"k":"v"}')
  })
})

describe('parseChatCompletionRequestBody', () => {
  it('parses full messages array including user and tool', () => {
    const raw = JSON.stringify({
      messages: [
        { role: 'user', content: 'hello' },
        {
          role: 'tool',
          tool_call_id: 'call_1',
          content: '{"ok":true}',
        },
      ],
    })
    const p = parseChatCompletionRequestBody(raw)
    expect(p?.messages).toHaveLength(2)
    expect(p?.messages[0].role).toBe('user')
    expect(p?.messages[0].content).toBe('hello')
    expect(p?.messages[1].role).toBe('tool')
    expect(p?.messages[1].tool_call_id).toBe('call_1')
    expect(p?.messages[1].content).toBe('{"ok":true}')
  })

  it('parses assistant tool_calls in conversation history', () => {
    const raw = JSON.stringify({
      messages: [
        {
          role: 'assistant',
          content: '',
          tool_calls: [
            {
              id: 'c1',
              type: 'function',
              function: { name: 'x', arguments: '{}' },
            },
          ],
        },
      ],
    })
    const p = parseChatCompletionRequestBody(raw)
    expect(p?.messages).toHaveLength(1)
    expect(p?.messages[0].tool_calls).toHaveLength(1)
    expect(p?.messages[0].tool_calls[0].name).toBe('x')
  })
})

describe('formatMessageContent', () => {
  it('joins multimodal text parts', () => {
    expect(
      formatMessageContent([
        { type: 'text', text: 'a' },
        { type: 'text', text: 'b' },
      ]),
    ).toBe('a\n\nb')
  })
})

describe('display flags', () => {
  it('responseHasStructuredDisplay', () => {
    expect(
      responseHasStructuredDisplay(
        parseChatCompletionResponseBody(
          JSON.stringify({
            choices: [{ message: { content: 'x' } }],
          }),
        ),
      ),
    ).toBe(true)
    expect(responseHasStructuredDisplay(null)).toBe(false)
  })

  it('requestHasStructuredDisplay', () => {
    expect(
      requestHasStructuredDisplay(
        parseChatCompletionRequestBody(
          JSON.stringify({
            messages: [{ role: 'tool', tool_call_id: 'c', content: 'z' }],
          }),
        ),
      ),
    ).toBe(true)
    expect(
      requestHasStructuredDisplay(
        parseChatCompletionRequestBody(
          JSON.stringify({
            messages: [{ role: 'user', content: 'hi' }],
          }),
        ),
      ),
    ).toBe(true)
  })
})
