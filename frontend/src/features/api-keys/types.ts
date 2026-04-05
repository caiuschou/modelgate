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
  model_allowlist: string[] | null
  ip_allowlist: string[] | null
  status: string
}

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
  model_allowlist?: string[]
  ip_allowlist?: string[]
}
