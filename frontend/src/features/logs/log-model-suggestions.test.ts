import { describe, expect, it } from 'vitest'
import type { OpenRouterModelRow } from '@/features/models/lib/openrouter-models'
import {
  buildLogModelSuggestions,
  filterOpenRouterModelsByQuery,
  sortOpenRouterRowsByRecentUsage,
  recentNameToLastUsedMap,
} from './log-model-suggestions'

const rows = (ids: string[]): OpenRouterModelRow[] =>
  ids.map((id) => ({ id, name: id.split('/').pop() }))

describe('log-model-suggestions', () => {
  it('sortOpenRouterRowsByRecentUsage orders by lastUsedAt desc', () => {
    const r = rows(['a/x', 'b/y', 'c/z'])
    const m = recentNameToLastUsedMap([
      { name: 'a/x', lastUsedAt: 100 },
      { name: 'c/z', lastUsedAt: 300 },
    ])
    const sorted = sortOpenRouterRowsByRecentUsage(r, m)
    expect(sorted.map((x) => x.id)).toEqual(['c/z', 'a/x', 'b/y'])
  })

  it('filterOpenRouterModelsByQuery matches id and name', () => {
    const all: OpenRouterModelRow[] = [
      { id: 'openai/gpt-4o', name: 'GPT-4o' },
      { id: 'anthropic/claude-3', name: 'Claude' },
    ]
    expect(filterOpenRouterModelsByQuery(all, 'gpt').map((x) => x.id)).toEqual([
      'openai/gpt-4o',
    ])
    expect(filterOpenRouterModelsByQuery(all, 'claude').map((x) => x.id)).toEqual([
      'anthropic/claude-3',
    ])
  })

  it('buildLogModelSuggestions: empty query shows recent only', () => {
    const r = buildLogModelSuggestions(
      [{ id: 'a/b' }],
      [{ name: 'x/y', lastUsedAt: 1 }],
      '  ',
    )
    expect(r.items).toHaveLength(1)
    expect(r.items[0]).toMatchObject({ kind: 'recent_only', name: 'x/y' })
  })

  it('buildLogModelSuggestions: filters catalog and sorts by recent', () => {
    const catalog: OpenRouterModelRow[] = [
      { id: 'm/a', name: 'A' },
      { id: 'm/b', name: 'B' },
    ]
    const recent = [
      { name: 'm/b', lastUsedAt: 200 },
      { name: 'm/a', lastUsedAt: 100 },
    ]
    const r = buildLogModelSuggestions(catalog, recent, 'm/')
    expect(r.items.map((i) => (i.kind === 'catalog' ? i.row.id : ''))).toEqual([
      'm/b',
      'm/a',
    ])
  })

  it('merges recent-only ids when not in OpenRouter catalog', () => {
    const catalog: OpenRouterModelRow[] = [{ id: 'openai/gpt-4o', name: 'G' }]
    const recent = [{ name: 'e2e_custom_xyz', lastUsedAt: 999_000 }]
    const r = buildLogModelSuggestions(catalog, recent, 'e2e_custom')
    expect(r.items.some((i) => i.kind === 'recent_only' && i.name === 'e2e_custom_xyz')).toBe(
      true,
    )
  })

  it('when catalog undefined, still filters recent by query', () => {
    const recent = [{ name: 'foo-bar', lastUsedAt: 1 }]
    const r = buildLogModelSuggestions(undefined, recent, 'foo')
    expect(r.items).toHaveLength(1)
    expect(r.items[0]).toMatchObject({ kind: 'recent_only', name: 'foo-bar' })
  })
})
