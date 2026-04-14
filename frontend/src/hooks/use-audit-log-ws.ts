import { useEffect, useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  applyAuditLogWebSocketMerge,
  cancelPendingAuditLogMergeTimers,
} from '@/features/logs/audit-log-ws-merge'
import { getApiBaseUrl } from '@/lib/runtime-config'
import { useAuthStore } from '@/stores/auth-store'
import { useAuditLogWsStore } from '@/stores/audit-log-ws-store'
import { useTeamStore } from '@/stores/team-store'

const AUDIT_LOG_UPDATED = 'audit_log_updated'

function buildLogsWsUrl(token: string, teamId: number | null): string {
  const enc = encodeURIComponent(token)
  const teamParam = teamId != null ? `&team_id=${teamId}` : ''
  const qs = `access_token=${enc}${teamParam}`
  const base = getApiBaseUrl().trim()
  if (base) {
    try {
      const u = new URL(base)
      u.protocol = u.protocol === 'https:' ? 'wss:' : 'ws:'
      u.pathname = '/api/v1/ws/logs'
      u.search = qs
      return u.toString()
    } catch {
      /* fall through to same-origin */
    }
  }
  const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${proto}//${window.location.host}/api/v1/ws/logs?${qs}`
}

/**
 * Subscribes to server push for audit log changes (list / detail / session threads).
 * Merges the updated row into list/detail caches and debounces thread refetches — avoids
 * invalidating entire lists (no full-table flash). Body queries for that request are
 * invalidated so open detail panels pick up new files.
 */
export function useAuditLogWebSocket(): void {
  const queryClient = useQueryClient()
  const token = useAuthStore((s) => s.token)
  const teamId = useTeamStore((s) => s.currentTeamId)
  const setConnected = useAuditLogWsStore((s) => s.setConnected)
  const reconnectRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (!token) {
      setConnected(false)
      return
    }
    const authToken = token

    let ws: WebSocket | null = null
    let cancelled = false

    const scheduleReconnect = () => {
      if (cancelled) return
      if (reconnectRef.current) clearTimeout(reconnectRef.current)
      reconnectRef.current = setTimeout(connect, 3000)
    }

    function connect() {
      if (cancelled) return
      try {
        ws = new WebSocket(buildLogsWsUrl(authToken, teamId))
      } catch {
        setConnected(false)
        scheduleReconnect()
        return
      }

      ws.onopen = () => {
        setConnected(true)
        if (reconnectRef.current) {
          clearTimeout(reconnectRef.current)
          reconnectRef.current = null
        }
      }
      ws.onclose = () => {
        setConnected(false)
        scheduleReconnect()
      }
      ws.onerror = () => {
        setConnected(false)
      }
      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data as string) as {
            type?: string
            request_id?: string
          }
          if (msg.type === AUDIT_LOG_UPDATED && msg.request_id) {
            const rid = msg.request_id
            applyAuditLogWebSocketMerge(queryClient, rid, () => {
              const id = useTeamStore.getState().currentTeamId
              return id ?? 'personal'
            })
          }
        } catch {
          /* ignore malformed */
        }
      }
    }

    connect()

    return () => {
      cancelled = true
      cancelPendingAuditLogMergeTimers()
      if (reconnectRef.current) clearTimeout(reconnectRef.current)
      reconnectRef.current = null
      ws?.close()
      setConnected(false)
    }
  }, [queryClient, token, teamId, setConnected])
}

export function useAuditLogWsConnected(): boolean {
  return useAuditLogWsStore((s) => s.connected)
}
