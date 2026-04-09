export interface AnalyticsSummary {
  total_requests: number
  success_requests: number
  total_tokens: number
  total_cost: number
  avg_latency_ms: number | null
}

export interface AnalyticsTimeBucket {
  bucket_start: number
  request_count: number
  total_tokens: number
  /** Sum of audit `cost` in this bucket (USD). */
  total_cost: number
  prompt_tokens: number
  completion_tokens: number
  /** Sum of `cached_prompt_tokens` (usage.prompt_tokens_details.cached_tokens). */
  cached_prompt_tokens: number
}

export interface AnalyticsModelSlice {
  model: string
  request_count: number
  total_tokens: number
}

export interface AnalyticsResponse {
  summary: AnalyticsSummary
  bucket_seconds: number
  series: AnalyticsTimeBucket[]
  by_model: AnalyticsModelSlice[]
}
