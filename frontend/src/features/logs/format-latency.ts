import { formatThreadActivitySpan } from '@/features/logs/format-thread-activity-span'

/** Display server-recorded latency (stored as milliseconds) as seconds in the UI. */
export function formatLatencySeconds(latencyMs: number | null | undefined): string {
  if (latencyMs == null || Number.isNaN(latencyMs)) return '—'
  const sec = latencyMs / 1000
  return sec.toLocaleString('zh-CN', {
    minimumFractionDigits: 0,
    maximumFractionDigits: 3,
  })
}

/**
 * Session total latency (sum of per-request `latency_ms`): ms below 1s, then 秒, then 分 / 时 / 天.
 */
export function formatCumulativeLatencyMs(ms: number | null | undefined): string {
  if (ms == null || Number.isNaN(ms)) return '—'
  const n = Math.max(0, ms)
  if (n < 1000) {
    return `${Math.round(n)} ms`
  }

  const totalSec = n / 1000
  if (totalSec < 60) {
    const rounded = Math.round(totalSec * 10) / 10
    const s = Number.isInteger(rounded)
      ? String(rounded)
      : rounded.toFixed(1).replace(/\.0$/, '')
    return `${s} 秒`
  }

  return formatThreadActivitySpan(Math.floor(totalSec))
}
