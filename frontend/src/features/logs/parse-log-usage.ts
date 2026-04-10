/** Parsed from upstream chat completion `usage` (JSON or final SSE chunk). */
export interface UsageData {
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  cost?: number
  is_byok?: boolean
  prompt_tokens_details?: {
    cached_tokens?: number
    cache_write_tokens?: number
    audio_tokens?: number
    video_tokens?: number
  }
  completion_tokens_details?: {
    reasoning_tokens?: number
    image_tokens?: number
    audio_tokens?: number
  }
  cost_details?: {
    upstream_inference_cost?: number
    upstream_inference_prompt_cost?: number
    upstream_inference_completions_cost?: number
  }
}

export function parseUsageFromBody(text: string): UsageData | null {
  try {
    const json = JSON.parse(text)
    if (json?.usage) return json.usage as UsageData
  } catch {
    /* not plain JSON — try SSE */
  }
  const lines = text.split('\n')
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim()
    if (!line.startsWith('data: ') || line === 'data: [DONE]') continue
    try {
      const chunk = JSON.parse(line.slice(6))
      if (chunk?.usage) return chunk.usage as UsageData
    } catch {
      continue
    }
  }
  return null
}
