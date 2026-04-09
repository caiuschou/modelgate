/** 24 UTC hour bucket starts used with `mergeSeriesTo24HourlyBars` / Recharts (newest = current hour start). */
export function hourBucketStarts24(endUnix: number): number[] {
  const hourFloor = Math.floor(endUnix / 3600) * 3600
  return Array.from({ length: 24 }, (_, i) => hourFloor - (23 - i) * 3600)
}

/**
 * API `start_time` / `end_time` for the same 24 buckets as `hourBucketStarts24(end)`.
 * Avoids `end - 86400`, which pulls in an extra hour (hourFloor−24) that the chart does not plot.
 */
export function analyticsRangeFor24HourlyBars(end: number): { start: number; end: number } {
  const hourFloor = Math.floor(end / 3600) * 3600
  return { start: hourFloor - 23 * 3600, end }
}
