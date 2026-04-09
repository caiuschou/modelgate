/** Format upstream USD cost for display (matches log detail usage). */
export function formatCostUsd(value: number): string {
  if (value === 0) return '$0'
  const abs = Math.abs(value)
  if (abs >= 1) return '$' + value.toFixed(2)
  if (abs >= 0.01) return '$' + value.toFixed(4)
  return '$' + value.toFixed(7)
}
