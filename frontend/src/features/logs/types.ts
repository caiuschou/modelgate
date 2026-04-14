export interface AuditLogListItem {
  request_id: string
  user_id: number | null
  token_id: number | null
  channel_id: string | null
  model: string | null
  request_type: string | null
  status_code: number | null
  error_message: string | null
  prompt_tokens: number | null
  completion_tokens: number | null
  cached_prompt_tokens: number | null
  total_tokens: number | null
  cost: number | null
  latency_ms: number | null
  /** Streaming: ms from first reasoning delta to first non-empty content delta. */
  reasoning_phase_ms?: number | null
  app_id: string | null
  thread_id: string | null
  finish_reason: string | null
  created_at: number
}

export interface AuditLogListResponse {
  data: AuditLogListItem[]
  total: number
  limit: number
  offset: number
}

/** One row per session (`thread_id`) from `GET /api/v1/logs/threads`. */
export interface AuditThreadListItem {
  thread_id: string
  user_id: number
  team_id: number | null
  first_seen_at: number
  last_seen_at: number
  request_count: number
}

export interface AuditThreadListResponse {
  data: AuditThreadListItem[]
  total: number
  limit: number
  offset: number
}

/** Full audit row returned by detail API (snake_case). */
export interface AuditLogRecord extends AuditLogListItem {
  request_body_path: string | null
  response_body_path: string | null
  metadata: Record<string, unknown> | null
}

export interface ExportCreateResponse {
  export_id: string
  status: string
  download_url: string
}
