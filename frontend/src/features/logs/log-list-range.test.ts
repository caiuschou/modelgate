import { endOfDay, startOfDay, subDays } from 'date-fns'
import { describe, expect, it } from 'vitest'
import {
  defaultLogListRange,
  endOfLocalDayUnix,
  normalizeLogListTimeRange,
  presetLogListRangeToday,
  presetLogListRangeYesterday,
  rollingLogListRange,
  startOfLocalDayUnix,
} from './log-list-range'

describe('endOfLocalDayUnix', () => {
  it('matches date-fns endOfDay in local timezone', () => {
    const d = new Date(2024, 5, 15, 14, 30, 0)
    expect(endOfLocalDayUnix(d)).toBe(Math.floor(endOfDay(d).getTime() / 1000))
  })
})

describe('startOfLocalDayUnix', () => {
  it('matches date-fns startOfDay in local timezone', () => {
    const d = new Date(2024, 5, 15, 14, 30, 0)
    expect(startOfLocalDayUnix(d)).toBe(Math.floor(startOfDay(d).getTime() / 1000))
  })
})

describe('defaultLogListRange', () => {
  it('uses end of local calendar day, not current unix time', () => {
    const noon = new Date(2024, 5, 15, 12, 0, 0)
    const { end, start } = defaultLogListRange(noon)
    const expectedEnd = Math.floor(endOfDay(noon).getTime() / 1000)
    expect(end).toBe(expectedEnd)
    expect(end).toBeGreaterThan(Math.floor(noon.getTime() / 1000))
    expect(start).toBe(end - 7 * 24 * 3600)
  })

  it('spans exactly 7×24 hours from start to end', () => {
    const { start, end } = defaultLogListRange(new Date(2025, 0, 1, 3, 0, 0))
    expect(end - start).toBe(7 * 24 * 3600)
  })
})

describe('rollingLogListRange', () => {
  it('matches default when dayCount is 7', () => {
    const at = new Date(2025, 3, 10, 15, 0, 0)
    expect(rollingLogListRange(7, at)).toEqual(defaultLogListRange(at))
  })
})

describe('presetLogListRangeToday', () => {
  it('is start/end of the same local day', () => {
    const at = new Date(2025, 2, 5, 18, 0, 0)
    const { start, end } = presetLogListRangeToday(at)
    expect(start).toBe(startOfLocalDayUnix(at))
    expect(end).toBe(endOfLocalDayUnix(at))
    expect(end - start).toBe(24 * 3600 - 1)
  })
})

describe('presetLogListRangeYesterday', () => {
  it('covers the previous local calendar day', () => {
    const at = new Date(2025, 2, 5, 18, 0, 0)
    const y = subDays(at, 1)
    const { start, end } = presetLogListRangeYesterday(at)
    expect(start).toBe(startOfLocalDayUnix(y))
    expect(end).toBe(endOfLocalDayUnix(y))
  })
})

describe('normalizeLogListTimeRange', () => {
  it('returns unchanged when start <= end', () => {
    expect(normalizeLogListTimeRange(100, 200, 'start')).toEqual({ start: 100, end: 200 })
  })

  it('snaps end to end of start day when start moved after end', () => {
    const start = endOfLocalDayUnix(new Date(2025, 0, 10, 12, 0, 0)) + 1
    const end = startOfLocalDayUnix(new Date(2025, 0, 5, 12, 0, 0))
    expect(start > end).toBe(true)
    const out = normalizeLogListTimeRange(start, end, 'start')
    expect(out.start).toBe(start)
    expect(out.end).toBe(endOfLocalDayUnix(new Date(start * 1000)))
  })

  it('snaps start to start of end day when end moved before start', () => {
    const end = startOfLocalDayUnix(new Date(2025, 0, 5, 12, 0, 0)) - 1
    const start = endOfLocalDayUnix(new Date(2025, 0, 10, 12, 0, 0))
    expect(start > end).toBe(true)
    const out = normalizeLogListTimeRange(start, end, 'end')
    expect(out.end).toBe(end)
    expect(out.start).toBe(startOfLocalDayUnix(new Date(end * 1000)))
  })
})
