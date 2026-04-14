import { describe, expect, it } from 'vitest'
import { formatCumulativeLatencyMs } from './format-latency'

describe('formatCumulativeLatencyMs', () => {
  it('null', () => {
    expect(formatCumulativeLatencyMs(null)).toBe('—')
  })
  it('milliseconds under 1s', () => {
    expect(formatCumulativeLatencyMs(0)).toBe('0 ms')
    expect(formatCumulativeLatencyMs(500)).toBe('500 ms')
    expect(formatCumulativeLatencyMs(999)).toBe('999 ms')
  })
  it('seconds under 1 minute', () => {
    expect(formatCumulativeLatencyMs(1000)).toBe('1 秒')
    expect(formatCumulativeLatencyMs(1500)).toBe('1.5 秒')
    expect(formatCumulativeLatencyMs(59900)).toBe('59.9 秒')
  })
  it('minutes and beyond', () => {
    expect(formatCumulativeLatencyMs(60_000)).toBe('1 分')
    expect(formatCumulativeLatencyMs(90_000)).toBe('1 分 30 秒')
    expect(formatCumulativeLatencyMs(3_600_000)).toBe('1 时')
  })
})
