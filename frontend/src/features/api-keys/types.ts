export interface ApiKeySummary {
  id: number
  preview: string
  created_at: number
  revoked: boolean
}

export interface ApiKeyListResponse {
  data: ApiKeySummary[]
}

export interface CreateMyApiKeyResponse {
  id: number
  api_key: string
  created_at: number
}
