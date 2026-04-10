import { endOfDay } from 'date-fns'

/** Local calendar end of day for `d`, Unix seconds (matches 结束时间 in `LogDateField`). */
export function endOfLocalDayUnix(d: Date): number {
  return Math.floor(endOfDay(d).getTime() / 1000)
}

/**
 * Default list window: end = local end of calendar day for `at`, start = 7×24h before that.
 * Pass a fixed `at` in unit tests; UI calls with no args (current day).
 */
export function defaultLogListRange(at: Date = new Date()): { start: number; end: number } {
  const end = endOfLocalDayUnix(at)
  return { start: end - 7 * 24 * 3600, end }
}
