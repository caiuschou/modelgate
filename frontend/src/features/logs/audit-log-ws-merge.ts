import type { QueryClient } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import type {
  AuditLogListItem,
  AuditLogListResponse,
  AuditLogRecord,
} from '@/features/logs/types'

/** Strip detail-only fields so list rows stay aligned with `GET /logs/request` list items. */
export function auditRecordToListItem(r: AuditLogRecord): AuditLogListItem {
  const {
    request_body_path,
    response_body_path,
    metadata,
    ...rest
  } = r
  void request_body_path
  void response_body_path
  void metadata
  return rest
}

let threadRefetchTimer: ReturnType<typeof setTimeout> | null = null
let analyticsInvalidateTimer: ReturnType<typeof setTimeout> | null = null

/**
 * Applies a single `audit_log_updated` event without invalidating whole list caches:
 * fetches the row, merges into list + detail caches, invalidates body text for that request,
 * and debounces session-thread refetches so aggregates stay correct without list-wide flashes.
 */
export function applyAuditLogWebSocketMerge(
  queryClient: QueryClient,
  requestId: string,
  getTeamKey: () => number | 'personal',
): void {
  void (async () => {
    let record: AuditLogRecord
    try {
      record = await apiClient
        .get(apiPath(`api/v1/logs/request/${encodeURIComponent(requestId)}`))
        .json<AuditLogRecord>()
    } catch {
      return
    }

    const listItem = auditRecordToListItem(record)
    const teamKey = getTeamKey()

    queryClient.setQueriesData<AuditLogRecord | undefined>(
      { queryKey: ['logs', 'detail', requestId], exact: false },
      () => record,
    )

    queryClient.setQueriesData<AuditLogListResponse | undefined>(
      { queryKey: ['logs', 'list', teamKey], exact: false },
      (old) => {
        if (!old?.data?.length) return old
        const idx = old.data.findIndex((r) => r.request_id === requestId)
        if (idx === -1) return old
        const data = [...old.data]
        data[idx] = listItem
        return { ...old, data }
      },
    )

    void queryClient.invalidateQueries({
      queryKey: ['logs', 'body', requestId],
      exact: false,
    })

    if (threadRefetchTimer) clearTimeout(threadRefetchTimer)
    threadRefetchTimer = setTimeout(() => {
      threadRefetchTimer = null
      const tk = getTeamKey()
      void queryClient.refetchQueries({
        predicate: (q) =>
          q.queryKey[0] === 'logs' &&
          q.queryKey[1] === 'threads' &&
          q.queryKey[2] === tk,
      })
    }, 450)

    if (analyticsInvalidateTimer) clearTimeout(analyticsInvalidateTimer)
    analyticsInvalidateTimer = setTimeout(() => {
      analyticsInvalidateTimer = null
      void queryClient.invalidateQueries({ queryKey: ['analytics'], exact: false })
    }, 1200)
  })()
}

export function cancelPendingAuditLogMergeTimers(): void {
  if (threadRefetchTimer) {
    clearTimeout(threadRefetchTimer)
    threadRefetchTimer = null
  }
  if (analyticsInvalidateTimer) {
    clearTimeout(analyticsInvalidateTimer)
    analyticsInvalidateTimer = null
  }
}
