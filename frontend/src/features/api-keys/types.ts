export interface ApiKeySummary {
  id: number
  name: string
  description: string
  preview: string
  created_at: number
  last_used_at: number | null
  revoked: boolean
  disabled: boolean
  expires_at: number | null
  quota_monthly_tokens: number | null
  quota_used_tokens: number
  max_concurrent_requests?: number | null
  quota_monthly_spend_minor?: string | null
  quota_used_spend_minor?: string
  model_allowlist: string[] | null
  ip_allowlist: string[] | null
  status: string
  team_id?: number | null
  /** When set, chat without `X-MG-Byok-Id` uses this BYOK; `null` = ModelGate `[upstream]`. */
  default_byok_profile_id?: number | null
  /** Session-level upstream affinity (RR + bindings); default false. */
  session_affinity_enabled?: boolean
  /** Ordered pool for affinity; omitted when unset. */
  upstream_pool?: UpstreamPoolEntry[] | null
}

export type UpstreamPoolEntry =
  | { kind: 'platform' }
  | { kind: 'byok'; byok_profile_id: number }

export interface ApiKeyListResponse {
  data: ApiKeySummary[]
}

export interface CreateMyApiKeyResponse {
  id: number
  api_key: string
  created_at: number
}

export interface CreateMyApiKeyBody {
  name: string
  description?: string
  expires_at?: number
  quota_monthly_tokens?: number
  max_concurrent_requests?: number
  quota_monthly_spend_minor?: string
  model_allowlist?: string[]
  ip_allowlist?: string[]
  /** Omit or leave unset for ModelGate `[upstream]`. */
  default_byok_profile_id?: number
}
