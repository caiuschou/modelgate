import type {
  ParsedChatCompletionRequest,
  ParsedChatCompletionResponse,
} from '@/features/logs/parse-chat-completion-display'

function isRecord(v: unknown): v is Record<string, unknown> {
  return v !== null && typeof v === 'object' && !Array.isArray(v)
}

function tryParseRecord(text: string): Record<string, unknown> | null {
  try {
    const j = JSON.parse(text) as unknown
    return isRecord(j) ? j : null
  } catch {
    return null
  }
}

/** UTF-8 byte length and line count for audit body preview. */
export function auditBodyPhysicalLine(raw: string): string {
  const bytes = new TextEncoder().encode(raw).length
  const lines = raw === '' ? 0 : raw.split(/\r\n|\r|\n/).length
  return `${formatBytes(bytes)} · ${lines.toLocaleString()} 行`
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const kb = bytes / 1024
  if (kb < 1024) return `${kb >= 10 ? kb.toFixed(0) : kb.toFixed(1)} KB`
  const mb = kb / 1024
  return `${mb.toFixed(1)} MB`
}

function jsonChoicesCount(text: string): number | null {
  const rec = tryParseRecord(text)
  if (!rec) return null
  const c = rec.choices
  if (!Array.isArray(c)) return null
  return c.length
}

/** Non-empty `data:` SSE lines (excluding `[DONE]`). */
export function countSseDataEvents(raw: string): number {
  let n = 0
  for (const line of raw.split('\n')) {
    const m = line.trim().match(/^data:\s*(.*)$/i)
    if (!m) continue
    const payload = (m[1] ?? '').trim()
    if (!payload || payload === '[DONE]') continue
    n++
  }
  return n
}

export function auditBodyRequestSemanticLine(
  raw: string,
  parsed: ParsedChatCompletionRequest | null,
): string | null {
  const rec = tryParseRecord(raw)
  const parts: string[] = []

  const msgCount =
    parsed && parsed.messages.length > 0
      ? parsed.messages.length
      : Array.isArray(rec?.messages)
        ? rec.messages.length
        : 0
  if (msgCount > 0) parts.push(`${msgCount} 条消息`)

  if (rec?.stream === true) parts.push('流式')

  const tools = rec?.tools
  if (Array.isArray(tools) && tools.length > 0) {
    parts.push(`${tools.length} 个工具`)
  }

  return parts.length ? parts.join(' · ') : null
}

export function auditBodyResponseSemanticLine(
  raw: string,
  parsed: ParsedChatCompletionResponse | null,
): string | null {
  const nChoices = jsonChoicesCount(raw)
  if (nChoices != null && nChoices > 0) {
    const parts: string[] = [`${nChoices} 个 choice`]
    if (parsed && parsed.tool_calls.length > 0) {
      parts.push(`${parsed.tool_calls.length} 次工具调用`)
    }
    return parts.join(' · ')
  }

  const sseN = countSseDataEvents(raw)
  if (sseN > 0) {
    const parts: string[] = [`${sseN} 条 SSE`]
    if (parsed && parsed.tool_calls.length > 0) {
      parts.push(`${parsed.tool_calls.length} 次工具调用`)
    }
    return parts.join(' · ')
  }

  if (parsed && parsed.tool_calls.length > 0) {
    return `${parsed.tool_calls.length} 次工具调用`
  }

  return null
}
