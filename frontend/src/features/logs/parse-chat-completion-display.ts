/**
 * Extract human-friendly slices from OpenAI-compatible chat completion
 * request/response bodies for audit log display.
 */

export interface ParsedToolCall {
  id: string | null
  name: string
  arguments: string
}

export interface ParsedChatCompletionResponse {
  /** Plain assistant text (non-tool). */
  content: string
  /** Extended thinking / reasoning text when present. */
  reasoning: string
  /** Model refused (OpenAI `refusal` field). */
  refusal: string
  tool_calls: ParsedToolCall[]
  /** How the payload was interpreted. */
  source: 'json' | 'sse_merged'
}

/** One entry from the chat completion request `messages` array. */
export interface ParsedChatMessage {
  role: string
  name: string | null
  content: string
  reasoning: string
  refusal: string
  tool_call_id: string | null
  tool_calls: ParsedToolCall[]
}

export interface ParsedChatCompletionRequest {
  /** Preserves API order. */
  messages: ParsedChatMessage[]
}

function isRecord(v: unknown): v is Record<string, unknown> {
  return v !== null && typeof v === 'object' && !Array.isArray(v)
}

function pickReasoning(m: Record<string, unknown>): string {
  const rc = m.reasoning_content
  if (typeof rc === 'string' && rc.trim()) return rc
  const r = m.reasoning
  if (typeof r === 'string' && r.trim()) return r
  return ''
}

export function formatMessageContent(content: unknown): string {
  if (content == null) return ''
  if (typeof content === 'string') return content
  if (!Array.isArray(content)) return ''
  const parts: string[] = []
  for (const part of content) {
    if (!isRecord(part)) continue
    const t = part.type
    if (t === 'text' && typeof part.text === 'string') {
      parts.push(part.text)
    }
  }
  return parts.join('\n\n')
}

function parseToolCallsFromMessage(raw: unknown): ParsedToolCall[] {
  if (!Array.isArray(raw)) return []
  const out: ParsedToolCall[] = []
  for (const tc of raw) {
    if (!isRecord(tc)) continue
    const id = typeof tc.id === 'string' ? tc.id : null
    const fn = tc.function
    let name = ''
    let args = ''
    if (isRecord(fn)) {
      if (typeof fn.name === 'string') name = fn.name
      if (typeof fn.arguments === 'string') args = fn.arguments
    }
    out.push({ id, name, arguments: args })
  }
  return out
}

function legacyFunctionCall(m: Record<string, unknown>): ParsedToolCall[] {
  const fc = m.function_call
  if (!isRecord(fc)) return []
  const name = typeof fc.name === 'string' ? fc.name : ''
  const args = typeof fc.arguments === 'string' ? fc.arguments : ''
  if (!name && !args) return []
  return [{ id: null, name, arguments: args }]
}

function parseAssistantMessage(m: Record<string, unknown>): ParsedChatCompletionResponse {
  const reasoning = pickReasoning(m)
  const content = formatMessageContent(m.content)
  const refusal = typeof m.refusal === 'string' ? m.refusal : ''
  let tool_calls = parseToolCallsFromMessage(m.tool_calls)
  if (tool_calls.length === 0) {
    tool_calls = legacyFunctionCall(m)
  }
  return {
    content,
    reasoning,
    refusal,
    tool_calls,
    source: 'json',
  }
}

function firstChoiceMessage(json: Record<string, unknown>): Record<string, unknown> | null {
  const choices = json.choices
  if (!Array.isArray(choices) || choices.length === 0) return null
  const ch0 = choices[0]
  if (!isRecord(ch0)) return null
  const msg = ch0.message
  if (isRecord(msg)) return msg
  return null
}

function parseJsonCompletionResponse(text: string): ParsedChatCompletionResponse | null {
  let json: unknown
  try {
    json = JSON.parse(text)
  } catch {
    return null
  }
  if (!isRecord(json)) return null
  const msg = firstChoiceMessage(json)
  if (!msg) return null
  return parseAssistantMessage(msg)
}

type ToolCallAcc = Map<
  number,
  { id: string | null; type: string; name: string; arguments: string }
>

function mergeToolCallDelta(toolCalls: unknown, acc: ToolCallAcc): void {
  if (!Array.isArray(toolCalls)) return
  for (const tc of toolCalls) {
    if (!isRecord(tc)) continue
    const idx = typeof tc.index === 'number' ? tc.index : 0
    let slot = acc.get(idx)
    if (!slot) {
      slot = {
        id: typeof tc.id === 'string' ? tc.id : null,
        type: typeof tc.type === 'string' ? tc.type : '',
        name: '',
        arguments: '',
      }
      acc.set(idx, slot)
    } else {
      if (typeof tc.id === 'string') slot.id = tc.id
      if (typeof tc.type === 'string') slot.type = tc.type
    }
    const fn = tc.function
    if (isRecord(fn)) {
      if (typeof fn.name === 'string' && fn.name) slot.name = fn.name
      if (typeof fn.arguments === 'string') slot.arguments += fn.arguments
    }
  }
}

function toolCallAccToList(acc: ToolCallAcc): ParsedToolCall[] {
  const entries = [...acc.entries()].sort(([a], [b]) => a - b)
  return entries.map(([, v]) => ({
    id: v.id,
    name: v.name,
    arguments: v.arguments,
  }))
}

function parseSseCompletionResponse(text: string): ParsedChatCompletionResponse | null {
  let content = ''
  let reasoning = ''
  let refusal = ''
  const toolAcc: ToolCallAcc = new Map()
  let sawData = false

  const lines = text.split('\n')
  for (const line of lines) {
    const t = line.trimEnd()
    const rest = t.startsWith('data: ') ? t.slice(6).trim() : ''
    if (!rest || rest === '[DONE]') continue
    let chunk: unknown
    try {
      chunk = JSON.parse(rest)
    } catch {
      continue
    }
    if (!isRecord(chunk)) continue
    const choices = chunk.choices
    if (!Array.isArray(choices) || choices.length === 0) continue
    const ch0 = choices[0]
    if (!isRecord(ch0)) continue

    const delta = ch0.delta
    const message = ch0.message
    if (isRecord(message)) {
      reasoning = pickReasoning(message) || reasoning
      content = formatMessageContent(message.content) || content
      if (typeof message.refusal === 'string' && message.refusal) {
        refusal = message.refusal
      }
      mergeToolCallDelta(message.tool_calls, toolAcc)
    }
    if (isRecord(delta)) {
      sawData = true
      if (typeof delta.content === 'string') content += delta.content
      if (typeof delta.reasoning_content === 'string') {
        reasoning += delta.reasoning_content
      }
      if (typeof delta.reasoning === 'string') {
        reasoning += delta.reasoning
      }
      if (typeof delta.refusal === 'string') {
        refusal = delta.refusal
      }
      mergeToolCallDelta(delta.tool_calls, toolAcc)
    }
  }

  if (!sawData && !content && !reasoning && toolAcc.size === 0 && !refusal) {
    return null
  }

  return {
    content,
    reasoning,
    refusal,
    tool_calls: toolCallAccToList(toolAcc),
    source: 'sse_merged',
  }
}

/** Parse chat completion **response** body (JSON or SSE). */
export function parseChatCompletionResponseBody(
  text: string,
): ParsedChatCompletionResponse | null {
  const trimmed = text.trim()
  if (!trimmed) return null
  const jsonTry = parseJsonCompletionResponse(trimmed)
  if (jsonTry) return jsonTry
  return parseSseCompletionResponse(text)
}

function parseRequestMessage(m: Record<string, unknown>): ParsedChatMessage {
  const role = typeof m.role === 'string' && m.role.trim() ? m.role : 'unknown'
  const name = typeof m.name === 'string' ? m.name : null
  const content = formatMessageContent(m.content)
  const reasoning = pickReasoning(m)
  const refusal = typeof m.refusal === 'string' ? m.refusal : ''
  const tool_call_id = typeof m.tool_call_id === 'string' ? m.tool_call_id : null
  let tool_calls = parseToolCallsFromMessage(m.tool_calls)
  if (tool_calls.length === 0) {
    tool_calls = legacyFunctionCall(m)
  }
  return {
    role,
    name,
    content,
    reasoning,
    refusal,
    tool_call_id,
    tool_calls,
  }
}

/** Parse chat completion **request** body (`messages` array). */
export function parseChatCompletionRequestBody(text: string): ParsedChatCompletionRequest | null {
  let json: unknown
  try {
    json = JSON.parse(text)
  } catch {
    return null
  }
  if (!isRecord(json)) return null
  const rawMessages = json.messages
  if (!Array.isArray(rawMessages) || rawMessages.length === 0) return null
  const messages: ParsedChatMessage[] = []
  for (const item of rawMessages) {
    if (!isRecord(item)) continue
    messages.push(parseRequestMessage(item))
  }
  if (messages.length === 0) return null
  return { messages }
}

export function responseHasStructuredDisplay(p: ParsedChatCompletionResponse | null): boolean {
  if (!p) return false
  return Boolean(
    p.content.trim() ||
      p.reasoning.trim() ||
      p.refusal.trim() ||
      p.tool_calls.length > 0,
  )
}

export function requestHasStructuredDisplay(p: ParsedChatCompletionRequest | null): boolean {
  if (!p) return false
  return p.messages.length > 0
}
