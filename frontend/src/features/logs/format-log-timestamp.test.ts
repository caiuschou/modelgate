import { describe, expect, it } from 'vitest'
import {
  formatLogTimestampHuman,
  formatLogTimestampRaw,
} from '@/features/logs/format-log-timestamp'

describe('formatLogTimestampRaw', () => {
  it('formats local wall time', () => {
    const ts = Math.floor(new Date('2026-03-08T14:05:06').getTime() / 1000)
    expect(formatLogTimestampRaw(ts)).toMatch(/^2026-03-08 14:05:06$/)
  })
})

describe('formatLogTimestampHuman', () => {
  it('uses relative wording within 7 calendar days', () => {
    const now = new Date('2026-04-10T15:00:00')
    const ts = Math.floor(new Date('2026-04-10T14:30:00').getTime() / 1000)
    const s = formatLogTimestampHuman(ts, now)
    expect(s.length).toBeGreaterThan(0)
    // zh-CN may be「约 29 分钟前」or「30 分钟内」depending on date-fns locale strings
    expect(s).toMatch(/分钟|小时|天|秒/)
  })

  it('uses calendar date when older than 7 days in the same year', () => {
    const now = new Date('2026-04-10T12:00:00')
    const ts = Math.floor(new Date('2026-03-01T09:00:00').getTime() / 1000)
    expect(formatLogTimestampHuman(ts, now)).toMatch(/3月1日/)
  })

  it('falls back to raw when timestamp is in the future', () => {
    const now = new Date('2026-04-10T12:00:00')
    const ts = Math.floor(new Date('2027-01-01T00:00:00').getTime() / 1000)
    expect(formatLogTimestampHuman(ts, now)).toBe(formatLogTimestampRaw(ts))
  })
})
