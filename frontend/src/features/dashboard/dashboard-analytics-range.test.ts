import { describe, expect, it } from 'vitest'
import {
  analyticsRangeFor24HourlyBars,
  hourBucketStarts24,
} from './dashboard-analytics-range'

describe('analyticsRangeFor24HourlyBars', () => {
  it('aligns API window with the 24 chart buckets (no stray hourFloor−24)', () => {
    const end = 1_700_000_000 // arbitrary
    const hourFloor = Math.floor(end / 3600) * 3600
    const { start, end: e } = analyticsRangeFor24HourlyBars(end)
    expect(e).toBe(end)
    expect(start).toBe(hourFloor - 23 * 3600)

    const buckets = hourBucketStarts24(end)
    expect(buckets).toHaveLength(24)
    expect(buckets[0]).toBe(start)
    expect(buckets[23]).toBe(hourFloor)
  })

  it('matches legacy mistake: end−86400 can start one hour earlier than first chart bar', () => {
    const end = 3_600 * 100 + 30 // not hour-aligned
    const hourFloor = Math.floor(end / 3600) * 3600
    const wrongStart = end - 24 * 3600
    const { start } = analyticsRangeFor24HourlyBars(end)
    expect(wrongStart).toBeLessThan(start)
    expect(start).toBe(hourFloor - 23 * 3600)
  })
})
