export type ByokProfileSummary = {
  id: number
  name: string
  base_url: string
  api_key_preview: string
  created_at: number
  updated_at: number
  revoked: boolean
}

export type ByokListResponse = {
  data: ByokProfileSummary[]
}

export type CreateByokBody = {
  name?: string
  base_url: string
  api_key: string
}

export type CreateByokResponse = {
  id: number
  created_at: number
}
