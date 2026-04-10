import { endOfDay } from 'date-fns'
import { describe, expect, it } from 'vitest'
import { defaultLogListRange, endOfLocalDayUnix } from './log-list-range'

describe('endOfLocalDayUnix', () => {
  it('matches date-fns endOfDay in local timezone', () => {
    const d = new Date(2024, 5, 15, 14, 30, 0)
    expect(endOfLocalDayUnix(d)).toBe(Math.floor(endOfDay(d).getTime() / 1000))
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
