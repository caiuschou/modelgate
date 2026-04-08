/** OpenRouter 公共模型目录（无需密钥；浏览器可直接请求，响应带 CORS *）。 */
export const OPENROUTER_MODELS_URL =
  'https://openrouter.ai/api/v1/models?output_modalities=all'

export type OpenRouterArchitecture = {
  modality?: string
  input_modalities?: string[]
  output_modalities?: string[]
  tokenizer?: string
  instruct_type?: string | null
}

export type OpenRouterModelRow = {
  id: string
  name?: string
  created?: number
  context_length?: number
  description?: string
  architecture?: OpenRouterArchitecture
  pricing?: { prompt?: string; completion?: string }
}

export type OpenRouterModelsResponse = {
  data: OpenRouterModelRow[]
}

export async function fetchOpenRouterModels(): Promise<OpenRouterModelRow[]> {
  const res = await fetch(OPENROUTER_MODELS_URL, {
    headers: { Accept: 'application/json' },
  })
  if (!res.ok) {
    throw new Error(`OpenRouter 模型列表请求失败（HTTP ${res.status}）`)
  }
  const body = (await res.json()) as OpenRouterModelsResponse
  if (!Array.isArray(body.data)) {
    throw new Error('OpenRouter 返回格式异常')
  }
  return body.data
}
