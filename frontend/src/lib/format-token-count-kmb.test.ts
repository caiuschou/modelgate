import { describe, expect, it } from 'vitest'
import { formatTokenCountKmb } from './format-token-count-kmb'

describe('formatTokenCountKmb', () => {
  it('nullish', () => {
    expect(formatTokenCountKmb(null)).toBe('—')
    expect(formatTokenCountKmb(undefined)).toBe('—')
  })
  it('under 1k uses locale grouping', () => {
    expect(formatTokenCountKmb(0)).toBe('0')
    expect(formatTokenCountKmb(999)).toBe('999')
  })
  it('thousands', () => {
    expect(formatTokenCountKmb(1000)).toBe('1k')
    expect(formatTokenCountKmb(1500)).toBe('1.5k')
    expect(formatTokenCountKmb(10_500)).toBe('10.5k')
    expect(formatTokenCountKmb(100_000)).toBe('100k')
    expect(formatTokenCountKmb(999_499)).toBe('999k')
    expect(formatTokenCountKmb(999_500)).toBe('1m')
  })
  it('rolls high thousands into millions', () => {
    expect(formatTokenCountKmb(999_999)).toBe('1m')
    expect(formatTokenCountKmb(1_000_000)).toBe('1m')
    expect(formatTokenCountKmb(1_500_000)).toBe('1.5m')
  })
  it('billions', () => {
    expect(formatTokenCountKmb(1_000_000_000)).toBe('1b')
    expect(formatTokenCountKmb(2_500_000_000)).toBe('2.5b')
  })
})
