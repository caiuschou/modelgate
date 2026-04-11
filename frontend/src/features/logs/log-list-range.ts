import { endOfDay, startOfDay, subDays } from 'date-fns'

/** Local calendar start of day for `d`, Unix seconds (matches 开始时间 in `LogDateField`). */
export function startOfLocalDayUnix(d: Date): number {
  return Math.floor(startOfDay(d).getTime() / 1000)
}

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

/** Same length semantics as `defaultLogListRange`, for arbitrary N×24h. */
export function rollingLogListRange(
  dayCount: number,
  at: Date = new Date(),
): { start: number; end: number } {
  const end = endOfLocalDayUnix(at)
  return { start: end - dayCount * 24 * 3600, end }
}

export function presetLogListRangeToday(at: Date = new Date()): { start: number; end: number } {
  const d = new Date(at)
  return { start: startOfLocalDayUnix(d), end: endOfLocalDayUnix(d) }
}

export function presetLogListRangeYesterday(at: Date = new Date()): { start: number; end: number } {
  const d = subDays(new Date(at), 1)
  return { start: startOfLocalDayUnix(d), end: endOfLocalDayUnix(d) }
}

/**
 * When the user picks a start after end (or end before start), snap the other bound to the
 * same local calendar day so the interval stays valid.
 */
export function normalizeLogListTimeRange(
  start: number,
  end: number,
  whichChanged: 'start' | 'end',
): { start: number; end: number } {
  if (start <= end) return { start, end }
  if (whichChanged === 'start') {
    return { start, end: endOfLocalDayUnix(new Date(start * 1000)) }
  }
  return { start: startOfLocalDayUnix(new Date(end * 1000)), end }
}
