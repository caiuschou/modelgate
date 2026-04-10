/** Query keys that live in the collapsible “更多条件” section (not keyword/model). */
export const LOG_LIST_ADVANCED_PARAM_KEYS = [
  'app_id',
  'finish_reason',
  'status_code',
  'token_id',
] as const

export function parseUnixSearchParam(raw: string | null, fallback: number): number {
  if (raw === null || raw === '') return fallback
  const n = Number(raw)
  return Number.isFinite(n) ? n : fallback
}

export function urlHasAdvancedFilters(sp: URLSearchParams): boolean {
  return LOG_LIST_ADVANCED_PARAM_KEYS.some((k) => {
    const v = sp.get(k)
    return v != null && v !== ''
  })
}

export type AuditLogListQueryInput = {
  startTime: number
  endTime: number
  limit: number
  offset: number
  keyword: string
  model: string
  appId: string
  finishReason: string
  statusCode: string
  tokenId: string
}

/** Builds the GET `api/v1/logs/request` query record from URL-derived fields. */
export function auditLogListQuery(
  input: AuditLogListQueryInput,
): Record<string, string | number> {
  const sc = input.statusCode.trim()
  const code = sc === '' ? NaN : Number(sc)
  const tid = input.tokenId.trim() === '' ? NaN : Number(input.tokenId.trim())
  return {
    start_time: input.startTime,
    end_time: input.endTime,
    limit: input.limit,
    offset: input.offset,
    ...(input.keyword.trim() ? { keyword: input.keyword.trim() } : {}),
    ...(input.model.trim() ? { model: input.model.trim() } : {}),
    ...(input.appId.trim() ? { app_id: input.appId.trim() } : {}),
    ...(input.finishReason.trim() ? { finish_reason: input.finishReason.trim() } : {}),
    ...(Number.isFinite(code) ? { status_code: code } : {}),
    ...(Number.isFinite(tid) ? { token_id: tid } : {}),
  }
}

export type BuildAppliedSearchParamsInput = {
  start: number
  end: number
  off: string
  kw: string
  m: string
  app: string
  fr: string
  sc: string
  tid: string
}

/** URL params written when the user applies filters (draft → URL). */
export function buildAppliedSearchParams(opts: BuildAppliedSearchParamsInput): URLSearchParams {
  const next = new URLSearchParams()
  next.set('start_time', String(opts.start))
  next.set('end_time', String(opts.end))
  next.set('offset', opts.off)
  if (opts.kw.trim()) next.set('keyword', opts.kw.trim())
  if (opts.m.trim()) next.set('model', opts.m.trim())
  if (opts.app.trim()) next.set('app_id', opts.app.trim())
  if (opts.fr.trim()) next.set('finish_reason', opts.fr.trim())
  if (opts.sc.trim()) next.set('status_code', opts.sc.trim())
  if (opts.tid.trim()) next.set('token_id', opts.tid.trim())
  return next
}
