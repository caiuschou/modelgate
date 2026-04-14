/**
 * Human-readable span from first to last activity (wall clock, unix seconds delta).
 */
export function formatThreadActivitySpan(totalSeconds: number): string {
  let t = Math.max(0, Math.floor(totalSeconds))
  if (t === 0) return '0 秒'

  const sec = t % 60
  t = Math.floor(t / 60)
  const min = t % 60
  t = Math.floor(t / 60)
  const hour = t % 24
  const day = Math.floor(t / 24)

  if (day > 0) {
    return hour > 0 ? `${day} 天 ${hour} 时` : `${day} 天`
  }
  if (hour > 0) {
    return min > 0 ? `${hour} 时 ${min} 分` : `${hour} 时`
  }
  if (min > 0) {
    return sec > 0 ? `${min} 分 ${sec} 秒` : `${min} 分`
  }
  return `${sec} 秒`
}
