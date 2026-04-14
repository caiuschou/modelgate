import { describe, expect, it } from 'vitest'
import { formatThreadActivitySpan } from './format-thread-activity-span'

describe('formatThreadActivitySpan', () => {
  it('zero', () => {
    expect(formatThreadActivitySpan(0)).toBe('0 秒')
  })
  it('seconds only', () => {
    expect(formatThreadActivitySpan(45)).toBe('45 秒')
  })
  it('minutes and seconds', () => {
    expect(formatThreadActivitySpan(90)).toBe('1 分 30 秒')
  })
  it('hours and minutes', () => {
    expect(formatThreadActivitySpan(3665)).toBe('1 时 1 分')
  })
  it('days', () => {
    expect(formatThreadActivitySpan(90000)).toBe('1 天 1 时')
  })
})
