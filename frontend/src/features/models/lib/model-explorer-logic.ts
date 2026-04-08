import type { OpenRouterModelRow } from '@/features/models/lib/openrouter-models'

export type ModalityChipId =
  | 'all'
  | 'text'
  | 'image'
  | 'embeddings'
  | 'audio'
  | 'video'
  | 'rerank'

export const MODALITY_CHIPS: {
  id: Exclude<ModalityChipId, 'all'>
  label: string
}[] = [
  { id: 'text', label: 'Text' },
  { id: 'image', label: 'Image' },
  { id: 'embeddings', label: 'Embeddings' },
  { id: 'audio', label: 'Audio' },
  { id: 'video', label: 'Video' },
  { id: 'rerank', label: 'Rerank' },
]

function norm(s: string): string {
  return s.trim().toLowerCase()
}

/** 用于 chips 计数与筛选的输出侧标签（与 OpenRouter 探索页语义对齐）。 */
export function getModelOutputTags(
  m: OpenRouterModelRow,
): Set<Exclude<ModalityChipId, 'all'>> {
  const tags = new Set<Exclude<ModalityChipId, 'all'>>()
  const outs = (m.architecture?.output_modalities ?? []).map(norm)
  const mod = norm(m.architecture?.modality ?? '')
  const id = norm(m.id)
  const name = norm(m.name ?? '')

  const has = (x: string) => outs.some((o) => o.includes(x))
  if (has('text')) tags.add('text')
  if (has('image')) tags.add('image')
  if (has('audio')) tags.add('audio')
  if (has('video')) tags.add('video')
  if (has('embed') || id.includes('embed') || name.includes('embed')) {
    tags.add('embeddings')
  }
  if (has('rerank') || id.includes('rerank') || name.includes('rerank')) {
    tags.add('rerank')
  }

  if (tags.size === 0) {
    if (mod.includes('embed')) tags.add('embeddings')
    else if (
      mod.includes('text') ||
      mod.includes('image') ||
      mod.includes('->')
    ) {
      if (mod.includes('image') && mod.includes('text')) {
        tags.add('text')
        tags.add('image')
      } else if (mod.includes('image')) tags.add('image')
      else tags.add('text')
    } else {
      tags.add('text')
    }
  }

  return tags
}

export function countByModality(
  rows: OpenRouterModelRow[],
): Record<Exclude<ModalityChipId, 'all'>, number> {
  const acc: Record<Exclude<ModalityChipId, 'all'>, number> = {
    text: 0,
    image: 0,
    embeddings: 0,
    audio: 0,
    video: 0,
    rerank: 0,
  }
  for (const m of rows) {
    for (const t of getModelOutputTags(m)) {
      acc[t] += 1
    }
  }
  return acc
}

export function filterBySearch(
  rows: OpenRouterModelRow[],
  q: string,
): OpenRouterModelRow[] {
  const n = norm(q)
  if (!n) return rows
  return rows.filter((m) => {
    const id = norm(m.id)
    const name = norm(m.name ?? '')
    const desc = norm(m.description ?? '')
    return id.includes(n) || name.includes(n) || desc.includes(n)
  })
}

export function filterByModality(
  rows: OpenRouterModelRow[],
  modality: ModalityChipId,
): OpenRouterModelRow[] {
  if (modality === 'all') return rows
  return rows.filter((m) => getModelOutputTags(m).has(modality))
}

export type ExplorerSort = 'newest' | 'name'

export function sortExplorerRows(
  rows: OpenRouterModelRow[],
  sort: ExplorerSort,
): OpenRouterModelRow[] {
  const copy = [...rows]
  if (sort === 'name') {
    copy.sort((a, b) =>
      (a.name ?? a.id).localeCompare(b.name ?? b.id, 'en'),
    )
  } else {
    copy.sort((a, b) => (b.created ?? 0) - (a.created ?? 0))
  }
  return copy
}

export type SeriesId = 'all' | 'gpt' | 'claude' | 'gemini' | 'other'

export function filterBySeries(
  rows: OpenRouterModelRow[],
  series: SeriesId,
): OpenRouterModelRow[] {
  if (series === 'all') return rows
  return rows.filter((m) => {
    const id = norm(m.id)
    const gpt = id.startsWith('openai/')
    const claude = id.startsWith('anthropic/')
    const gemini = id.startsWith('google/')
    if (series === 'gpt') return gpt
    if (series === 'claude') return claude
    if (series === 'gemini') return gemini
    return !gpt && !claude && !gemini
  })
}

export function filterByContextMin(
  rows: OpenRouterModelRow[],
  minCtx: number | null,
): OpenRouterModelRow[] {
  if (minCtx == null || minCtx <= 0) return rows
  return rows.filter(
    (m) => m.context_length != null && m.context_length >= minCtx,
  )
}

export type InputModalityFilterId = 'text' | 'image' | 'audio' | 'video'

export const INPUT_MODALITY_OPTIONS: { id: InputModalityFilterId; label: string }[] =
  [
    { id: 'text', label: 'Text' },
    { id: 'image', label: 'Image' },
    { id: 'audio', label: 'Audio' },
    { id: 'video', label: 'Video' },
  ]

/** 模型须同时支持所选的全部输入模态（与 OpenRouter 侧栏「Input Modalities」语义接近）。 */
export function filterByRequiredInputModalities(
  rows: OpenRouterModelRow[],
  required: Set<string>,
): OpenRouterModelRow[] {
  if (required.size === 0) return rows
  return rows.filter((m) => {
    const ins = new Set(
      (m.architecture?.input_modalities ?? []).map((x) => norm(x)),
    )
    for (const r of required) {
      if (!ins.has(norm(r))) return false
    }
    return true
  })
}

export function formatContextShort(n: number | undefined): string {
  if (n == null) return '—'
  if (n >= 1_000_000) return `${Math.round(n / 1_000_000)}M context`
  if (n >= 1000) return `${Math.round(n / 1000)}K context`
  return `${n} context`
}

/** OpenRouter `pricing.prompt` 为每 token 美元；乘 1e6 得「每百万 input token 美元」。 */
export function formatInputPricePerM(prompt?: string): string {
  if (prompt == null || prompt === '') return '—'
  const v = Number(prompt)
  if (!Number.isFinite(v)) return '—'
  const perM = v * 1_000_000
  if (perM === 0) return '免费'
  if (perM >= 100) return `$${perM.toFixed(0)}/M in`
  if (perM >= 10) return `$${perM.toFixed(1)}/M in`
  if (perM >= 1) return `$${perM.toFixed(2)}/M in`
  return `$${perM.toFixed(3)}/M in`
}

export function formatCreatedLabel(created?: number): string {
  if (created == null) return ''
  const d = new Date(created * 1000)
  return d.toLocaleDateString()
}
