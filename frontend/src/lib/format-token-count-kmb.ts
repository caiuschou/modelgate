/** Compact token counts for dense tables: `999`, `1.2k`, `10k`, `3.4m`, `1b` (lowercase). */
export function formatTokenCountKmb(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—'
  const v = Math.trunc(Number(n))
  if (v < 0) return v.toLocaleString()
  if (v < 1000) return v.toLocaleString()

  let d = 1000
  while (d < 1_000_000_000 && v >= d * 1000) {
    d *= 1000
  }
  /** Avoid `1000k`-style rounding at tier boundaries (e.g. 999,500 → `1m`). */
  while (d < 1_000_000_000 && Math.round(v / d) >= 1000) {
    d *= 1000
  }
  const scaled = v / d
  const suf: 'k' | 'm' | 'b' =
    d === 1_000_000_000 ? 'b' : d === 1_000_000 ? 'm' : 'k'

  if (scaled >= 100) {
    return `${Math.round(scaled)}${suf}`
  }
  const rounded1 = Math.round(scaled * 10) / 10
  if (Number.isInteger(rounded1)) {
    return `${rounded1}${suf}`
  }
  const t = rounded1.toFixed(1)
  return `${t.endsWith('.0') ? t.slice(0, -2) : t}${suf}`
}
