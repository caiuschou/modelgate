/** Display server-recorded latency (stored as milliseconds) as seconds in the UI. */
export function formatLatencySeconds(latencyMs: number | null | undefined): string {
  if (latencyMs == null || Number.isNaN(latencyMs)) return '—'
  const sec = latencyMs / 1000
  return sec.toLocaleString('zh-CN', {
    minimumFractionDigits: 0,
    maximumFractionDigits: 3,
  })
}
