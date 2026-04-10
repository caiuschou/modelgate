import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  filterRecentModelsByQuery,
  readRecentLogModels,
  rememberLogModel,
} from './log-recent-models'

const STORAGE_KEY = 'modelgate.logRecentModels'

describe('log-recent-models', () => {
  beforeEach(() => {
    localStorage.removeItem(STORAGE_KEY)
  })

  afterEach(() => {
    localStorage.removeItem(STORAGE_KEY)
  })

  it('rememberLogModel updates lastUsedAt and orders by recency', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-01-01T12:00:00.000Z'))
    rememberLogModel('a')
    vi.advanceTimersByTime(60_000)
    rememberLogModel('b')
    vi.advanceTimersByTime(60_000)
    rememberLogModel('a')
    const entries = readRecentLogModels()
    expect(entries.map((e) => e.name)).toEqual(['a', 'b'])
    expect(entries[0].lastUsedAt).toBeGreaterThan(entries[1].lastUsedAt)
    vi.useRealTimers()
  })

  it('migrates legacy string array to entries with decreasing times', () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(['first', 'second']))
    const entries = readRecentLogModels()
    expect(entries.map((e) => e.name)).toEqual(['first', 'second'])
    expect(entries[0].lastUsedAt).toBeGreaterThanOrEqual(entries[1].lastUsedAt)
  })

  it('readRecentLogModels returns empty when storage missing', () => {
    expect(readRecentLogModels()).toEqual([])
  })

  it('filterRecentModelsByQuery is case-insensitive substring', () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify([
        { name: 'GPT-4o', lastUsedAt: 100 },
        { name: 'claude-3', lastUsedAt: 200 },
      ]),
    )
    const all = readRecentLogModels()
    expect(filterRecentModelsByQuery(all, 'gpt').map((e) => e.name)).toEqual(['GPT-4o'])
    expect(filterRecentModelsByQuery(all, '').map((e) => e.name)).toEqual(['claude-3', 'GPT-4o'])
  })
})
