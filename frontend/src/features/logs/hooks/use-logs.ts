import { useMutation, useQuery } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import type {
  AuditLogListResponse,
  AuditLogRecord,
  ExportCreateResponse,
} from '@/features/logs/types'

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

export function useAuditLogList(query: Record<string, string | number | undefined | null>) {
  const search = toSearchParams(query)
  return useQuery({
    queryKey: ['logs', 'list', search.toString()],
    queryFn: async () => {
      const path = apiPath('api/v1/logs/request')
      const url = search.toString() ? `${path}?${search}` : path
      return apiClient.get(url).json<AuditLogListResponse>()
    },
    staleTime: 15_000,
  })
}

export function useAuditLogDetail(requestId: string | undefined) {
  return useQuery({
    queryKey: ['logs', 'detail', requestId],
    queryFn: () =>
      apiClient.get(apiPath(`api/v1/logs/request/${requestId}`)).json<AuditLogRecord>(),
    enabled: Boolean(requestId),
    staleTime: 15_000,
  })
}

export function useExportAuditLogs() {
  return useMutation({
    mutationFn: async (body: {
      start_time?: number
      end_time?: number
      format?: string
    }) =>
      apiClient
        .post(apiPath('api/v1/logs/export'), { json: body })
        .json<ExportCreateResponse>(),
  })
}

export async function downloadExportFile(downloadUrl: string): Promise<Blob> {
  const normalized = downloadUrl.startsWith('/')
    ? downloadUrl.slice(1)
    : downloadUrl
  return apiClient.get(apiPath(normalized)).blob()
}
