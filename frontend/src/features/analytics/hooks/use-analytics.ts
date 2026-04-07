import { useQuery } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import type { AnalyticsResponse } from '@/features/analytics/types'

function toSearchParams(
  record: Record<string, string | number | undefined | null>,
): URLSearchParams {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(record)) {
    if (v === undefined || v === null || v === '') continue
    p.set(k, String(v))
  }
  return p
}

export function useAnalytics(query: Record<string, string | number | undefined | null>) {
  const search = toSearchParams(query)
  return useQuery({
    queryKey: ['analytics', search.toString()],
    queryFn: async () => {
      const path = apiPath('api/v1/analytics')
      const url = search.toString() ? `${path}?${search}` : path
      return apiClient.get(url).json<AnalyticsResponse>()
    },
    staleTime: 15_000,
  })
}
