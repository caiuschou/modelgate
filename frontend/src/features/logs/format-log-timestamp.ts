import {
  differenceInCalendarDays,
  format,
  formatDistance,
  isSameYear,
} from 'date-fns'
import { zhCN } from 'date-fns/locale'

/** Full local timestamp for tooltips and fallbacks (e.g. clock skew). */
export function formatLogTimestampRaw(unixSeconds: number): string {
  return format(new Date(unixSeconds * 1000), 'yyyy-MM-dd HH:mm:ss', {
    locale: zhCN,
  })
}

/**
 * Short label for lists and detail: relative within ~7 calendar days, else calendar date.
 */
export function formatLogTimestampHuman(
  unixSeconds: number,
  now: Date = new Date(),
): string {
  const d = new Date(unixSeconds * 1000)
  if (d.getTime() > now.getTime()) {
    return formatLogTimestampRaw(unixSeconds)
  }
  const calendarDays = differenceInCalendarDays(now, d)
  if (calendarDays < 7) {
    return formatDistance(now, d, { addSuffix: true, locale: zhCN })
  }
  if (isSameYear(d, now)) {
    return format(d, 'M月d日 HH:mm', { locale: zhCN })
  }
  return format(d, 'yyyy年M月d日 HH:mm', { locale: zhCN })
}
