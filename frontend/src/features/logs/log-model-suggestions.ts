import type { OpenRouterModelRow } from '@/features/models/lib/openrouter-models'
import type { RecentModelEntry } from '@/features/logs/log-recent-models'

/** 全目录匹配结果过多时只展示前 N 条，避免卡顿 */
export const LOG_MODEL_SUGGEST_CAP = 200

export function recentNameToLastUsedMap(
  entries: RecentModelEntry[],
): Map<string, number> {
  const m = new Map<string, number>()
  for (const e of entries) {
    m.set(e.name, e.lastUsedAt)
  }
  return m
}

export function sortOpenRouterRowsByRecentUsage(
  rows: OpenRouterModelRow[],
  recentMap: Map<string, number>,
): OpenRouterModelRow[] {
  return [...rows].sort((a, b) => {
    const ta = recentMap.get(a.id) ?? 0
    const tb = recentMap.get(b.id) ?? 0
    if (tb !== ta) return tb - ta
    return a.id.localeCompare(b.id)
  })
}

/** 按 id / display name 子串匹配（不区分大小写） */
export function filterOpenRouterModelsByQuery(
  all: OpenRouterModelRow[],
  q: string,
): OpenRouterModelRow[] {
  const n = q.trim().toLowerCase()
  if (!n) return []
  return all.filter((m) => {
    if (m.id.toLowerCase().includes(n)) return true
    const name = m.name?.toLowerCase() ?? ''
    return name.includes(n)
  })
}

function filterRecentByQuery(
  recent: RecentModelEntry[],
  q: string,
): RecentModelEntry[] {
  const n = q.trim().toLowerCase()
  if (!n) return []
  return recent.filter((e) => e.name.toLowerCase().includes(n))
}

function suggestionItemTimeMs(item: LogModelSuggestionItem): number {
  if (item.kind === 'catalog') {
    return item.lastUsedAt ?? 0
  }
  return item.lastUsedAt
}

function suggestionItemSortKey(item: LogModelSuggestionItem): string {
  return item.kind === 'catalog' ? item.row.id : item.name
}

function mergeSuggestionItems(
  a: LogModelSuggestionItem,
  b: LogModelSuggestionItem,
): number {
  const ta = suggestionItemTimeMs(a)
  const tb = suggestionItemTimeMs(b)
  if (tb !== ta) return tb - ta
  return suggestionItemSortKey(a).localeCompare(suggestionItemSortKey(b))
}

export type LogModelSuggestionItem =
  | {
      kind: 'catalog'
      row: OpenRouterModelRow
      lastUsedAt: number | null
    }
  | { kind: 'recent_only'; name: string; lastUsedAt: number }

export type BuildLogModelSuggestionsResult = {
  items: LogModelSuggestionItem[]
  /** 输入非空但目录与最近记录均无匹配时，可提供「使用当前输入」 */
  showUseCustom: boolean
  catalogTruncated: boolean
}

/**
 * - 输入为空：仅展示最近使用记录（最多 30 条），便于快速点选。
 * - 输入非空：从 OpenRouter 全目录筛选，并与「最近使用」中匹配的自定义 id 合并；
 *   统一按最近使用时间排序（未在本地用过的目录模型 time=0，靠后）。
 */
export function buildLogModelSuggestions(
  catalog: OpenRouterModelRow[] | undefined,
  recent: RecentModelEntry[],
  mainQuery: string,
): BuildLogModelSuggestionsResult {
  const q = mainQuery.trim()
  const recentMap = recentNameToLastUsedMap(recent)

  if (q === '') {
    const items: LogModelSuggestionItem[] = recent.slice(0, 30).map((e) => ({
      kind: 'recent_only' as const,
      name: e.name,
      lastUsedAt: e.lastUsedAt,
    }))
    return {
      items,
      showUseCustom: false,
      catalogTruncated: false,
    }
  }

  const recentHits = filterRecentByQuery(recent, q)

  if (catalog === undefined) {
    const items: LogModelSuggestionItem[] = recentHits.map((e) => ({
      kind: 'recent_only' as const,
      name: e.name,
      lastUsedAt: e.lastUsedAt,
    }))
    return {
      items,
      showUseCustom: q.length > 0 && items.length === 0,
      catalogTruncated: false,
    }
  }

  let catalogMatched = filterOpenRouterModelsByQuery(catalog, q)
  catalogMatched = sortOpenRouterRowsByRecentUsage(catalogMatched, recentMap)
  const catalogIds = new Set(catalogMatched.map((r) => r.id))

  const recentOnlyExtra = recentHits.filter((e) => !catalogIds.has(e.name))

  let items: LogModelSuggestionItem[] = [
    ...catalogMatched.map((row) => ({
      kind: 'catalog' as const,
      row,
      lastUsedAt: recentMap.get(row.id) ?? null,
    })),
    ...recentOnlyExtra.map((e) => ({
      kind: 'recent_only' as const,
      name: e.name,
      lastUsedAt: e.lastUsedAt,
    })),
  ]

  items.sort(mergeSuggestionItems)

  let catalogTruncated = false
  if (items.length > LOG_MODEL_SUGGEST_CAP) {
    items = items.slice(0, LOG_MODEL_SUGGEST_CAP)
    catalogTruncated = true
  }

  const showUseCustom = q.length > 0 && items.length === 0

  return { items, showUseCustom, catalogTruncated }
}
