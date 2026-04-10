import { useMemo } from 'react'
import { Link, useParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { VirtualizedLogBody } from '@/features/logs/components/virtualized-log-body'
import { useMyByokProfiles } from '@/features/byok/hooks/use-byok-profiles'
import {
  useAuditLogBody,
  useAuditLogDetail,
} from '@/features/logs/hooks/use-logs'
import type { AuditLogRecord } from '@/features/logs/types'
import { formatCostUsd } from '@/lib/format-cost'

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

function formatBodyText(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2)
  } catch {
    return raw
  }
}

interface UsageData {
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  cost?: number
  is_byok?: boolean
  prompt_tokens_details?: {
    cached_tokens?: number
    cache_write_tokens?: number
    audio_tokens?: number
    video_tokens?: number
  }
  completion_tokens_details?: {
    reasoning_tokens?: number
    image_tokens?: number
    audio_tokens?: number
  }
  cost_details?: {
    upstream_inference_cost?: number
    upstream_inference_prompt_cost?: number
    upstream_inference_completions_cost?: number
  }
}

function parseUsageFromBody(text: string): UsageData | null {
  try {
    const json = JSON.parse(text)
    if (json?.usage) return json.usage as UsageData
  } catch {
    /* not plain JSON — try SSE */
  }
  const lines = text.split('\n')
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim()
    if (!line.startsWith('data: ') || line === 'data: [DONE]') continue
    try {
      const chunk = JSON.parse(line.slice(6))
      if (chunk?.usage) return chunk.usage as UsageData
    } catch {
      continue
    }
  }
  return null
}

/** Strip bulky header maps from the raw metadata JSON block (shown separately above). */
function metadataWithoutHeaders(
  metadata: AuditLogRecord['metadata'],
): Record<string, unknown> | null {
  if (!metadata || typeof metadata !== 'object' || Array.isArray(metadata)) {
    return null
  }
  const m = { ...(metadata as Record<string, unknown>) }
  delete m.request_headers
  delete m.response_headers
  return Object.keys(m).length > 0 ? m : null
}

function parseHeadersFromMetadata(raw: unknown): Record<string, string> | null {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return null
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(raw)) {
    if (typeof v === 'string') out[k] = v
    else if (v != null && typeof v !== 'object') out[k] = String(v)
  }
  return Object.keys(out).length > 0 ? out : null
}

function AuditHeadersPanel({
  title,
  headers,
}: {
  title: string
  headers: Record<string, string> | null
}) {
  const entries = headers
    ? Object.entries(headers).sort(([a], [b]) => a.localeCompare(b))
    : []

  return (
    <Card className="p-4">
      <h2 className="text-sm font-medium">{title}</h2>
      <p className="mt-1 text-xs text-muted-foreground">
        敏感头（如 Authorization、Cookie）在审计记录中为{' '}
        <span className="font-mono">[REDACTED]</span>。
      </p>
      {entries.length === 0 ? (
        <p className="mt-3 text-sm text-muted-foreground">无记录</p>
      ) : (
        <dl className="mt-3 max-h-64 space-y-2 overflow-y-auto rounded-md border border-border/60 bg-muted/20 p-3">
          {entries.map(([name, value]) => (
            <div key={name}>
              <dt className="font-mono text-xs text-muted-foreground">{name}</dt>
              <dd className="mt-0.5 break-all font-mono text-xs">{value}</dd>
            </div>
          ))}
        </dl>
      )}
    </Card>
  )
}

/** Chat audit metadata from proxy (`chat_audit_metadata`). */
function parseAuditUpstreamMeta(
  metadata: AuditLogRecord['metadata'],
): { isByok: boolean | null; profileId: number | null } {
  if (!metadata) return { isByok: null, profileId: null }
  const isByokRaw = metadata['is_byok']
  const pidRaw = metadata['byok_profile_id']
  const isByok =
    typeof isByokRaw === 'boolean'
      ? isByokRaw
      : null
  let profileId: number | null = null
  if (typeof pidRaw === 'number' && Number.isInteger(pidRaw) && pidRaw > 0) {
    profileId = pidRaw
  }
  return { isByok, profileId }
}

function TokenDetailRow({ label, value }: { label: string; value?: number }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={`font-mono text-xs ${value === 0 ? 'text-muted-foreground' : ''}`}>
        {value?.toLocaleString() ?? '—'}
      </span>
    </div>
  )
}

function CostDetailRow(props: {
  label: string
  value?: number
  bold?: boolean
  indent?: boolean
}) {
  const { label, value, bold, indent } = props
  return (
    <div
      className={`flex items-center justify-between gap-4${indent ? ' pl-3' : ''}`}
    >
      <span className={`text-xs ${bold ? 'font-medium' : 'text-muted-foreground'}`}>
        {label}
      </span>
      <span className={`font-mono text-xs${bold ? ' font-medium' : ''}`}>
        {value != null ? formatCostUsd(value) : '—'}
      </span>
    </div>
  )
}

function UsageDetailCard({
  usage,
  auditUpstream,
}: {
  usage: UsageData
  auditUpstream: { isByok: boolean | null; profileId: number | null }
}) {
  const { prompt_tokens, completion_tokens, total_tokens } = usage
  const promptPct =
    total_tokens > 0 ? (prompt_tokens / total_tokens) * 100 : 0
  const completionPct =
    total_tokens > 0 ? (completion_tokens / total_tokens) * 100 : 0

  const pd = usage.prompt_tokens_details
  const cd = usage.completion_tokens_details
  const costD = usage.cost_details

  const upstreamTag =
    auditUpstream.isByok ?? usage.is_byok ?? null

  return (
    <Card className="space-y-4 p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-medium">用量与成本</h2>
        {upstreamTag != null && (
          <span
            className={`rounded-full px-2 py-0.5 text-xs font-medium ${
              upstreamTag
                ? 'bg-blue-500/10 text-blue-600 dark:text-blue-400'
                : 'bg-muted text-muted-foreground'
            }`}
          >
            {upstreamTag ? 'BYOK' : '平台上游'}
          </span>
        )}
      </div>

      {/* Token overview with proportion bar */}
      <div className="space-y-2">
        <div className="flex items-baseline gap-6 text-sm">
          <span>
            <span className="text-muted-foreground">输入 </span>
            <span className="font-mono font-medium">
              {prompt_tokens.toLocaleString()}
            </span>
          </span>
          <span>
            <span className="text-muted-foreground">输出 </span>
            <span className="font-mono font-medium">
              {completion_tokens.toLocaleString()}
            </span>
          </span>
          <span>
            <span className="text-muted-foreground">合计 </span>
            <span className="font-mono font-medium">
              {total_tokens.toLocaleString()}
            </span>
          </span>
        </div>

        {total_tokens > 0 && (
          <>
            <div className="flex h-2 w-full overflow-hidden rounded-full bg-muted">
              <div
                className="bg-sky-500 transition-all"
                style={{ width: `${promptPct}%` }}
              />
              <div
                className="bg-emerald-500 transition-all"
                style={{ width: `${completionPct}%` }}
              />
            </div>
            <div className="flex gap-4 text-xs text-muted-foreground">
              <span className="flex items-center gap-1.5">
                <span className="inline-block h-2 w-2 rounded-full bg-sky-500" />
                输入 {promptPct.toFixed(0)}%
              </span>
              <span className="flex items-center gap-1.5">
                <span className="inline-block h-2 w-2 rounded-full bg-emerald-500" />
                输出 {completionPct.toFixed(0)}%
              </span>
            </div>
          </>
        )}
      </div>

      {/* Token detail breakdowns (two columns) */}
      {(pd || cd) && (
        <div className="grid gap-3 sm:grid-cols-2">
          {pd && (
            <div className="space-y-1.5 rounded-lg border border-border/60 p-3">
              <p className="text-xs font-medium text-muted-foreground">
                输入 Token 明细
              </p>
              <TokenDetailRow label="缓存命中" value={pd.cached_tokens} />
              <TokenDetailRow label="缓存写入" value={pd.cache_write_tokens} />
              <TokenDetailRow label="音频" value={pd.audio_tokens} />
              <TokenDetailRow label="视频" value={pd.video_tokens} />
            </div>
          )}
          {cd && (
            <div className="space-y-1.5 rounded-lg border border-border/60 p-3">
              <p className="text-xs font-medium text-muted-foreground">
                输出 Token 明细
              </p>
              <TokenDetailRow label="推理" value={cd.reasoning_tokens} />
              <TokenDetailRow label="图片" value={cd.image_tokens} />
              <TokenDetailRow label="音频" value={cd.audio_tokens} />
            </div>
          )}
        </div>
      )}

      {/* Cost breakdown */}
      {(usage.cost != null || costD) && (
        <div className="space-y-1.5 rounded-lg border border-border/60 p-3">
          <p className="text-xs font-medium text-muted-foreground">成本明细</p>
          {usage.cost != null && (
            <CostDetailRow label="总成本" value={usage.cost} bold />
          )}
          {costD && (
            <>
              <CostDetailRow
                label="上游推理成本"
                value={costD.upstream_inference_cost}
              />
              <CostDetailRow
                label="输入"
                value={costD.upstream_inference_prompt_cost}
                indent
              />
              <CostDetailRow
                label="输出"
                value={costD.upstream_inference_completions_cost}
                indent
              />
            </>
          )}
        </div>
      )}
    </Card>
  )
}

function AuditBodyPanel(props: {
  title: string
  requestId: string
  part: 'request' | 'response'
  hasPath: boolean
  emptyHint: string
  streamSseNote?: boolean
}) {
  const { title, requestId, part, hasPath, emptyHint, streamSseNote } = props
  const q = useAuditLogBody(requestId, part, hasPath)
  const formatted = q.data !== undefined ? formatBodyText(q.data) : ''

  return (
    <Card className="space-y-3 p-4">
      <h2 className="text-sm font-medium">{title}</h2>
      {streamSseNote && (
        <p className="text-xs text-muted-foreground">
          原始 SSE（与客户端收到的 event-stream 一致）。
        </p>
      )}
      {!hasPath && (
        <p className="text-sm text-muted-foreground">{emptyHint}</p>
      )}
      {hasPath && q.isLoading && (
        <p className="text-sm text-muted-foreground">加载中…</p>
      )}
      {hasPath && q.isError && (
        <p className="text-sm text-red-600" role="alert">
          无法加载正文（文件不存在或已清理）。
        </p>
      )}
      {hasPath && q.data !== undefined && <VirtualizedLogBody text={formatted} />}
    </Card>
  )
}

export function LogDetailPage() {
  const { requestId } = useParams<{ requestId: string }>()
  const decoded = requestId ? decodeURIComponent(requestId) : ''
  const { data, isLoading, isError } = useAuditLogDetail(decoded || undefined)
  const { data: byokRes } = useMyByokProfiles()
  const responseBody = useAuditLogBody(
    decoded || undefined,
    'response',
    Boolean(data?.response_body_path),
  )
  const usage = useMemo(() => {
    if (!responseBody.data) return null
    return parseUsageFromBody(responseBody.data)
  }, [responseBody.data])

  const auditUpstream = useMemo(
    () => parseAuditUpstreamMeta(data?.metadata ?? null),
    [data?.metadata],
  )

  const requestHeaders = useMemo(
    () => parseHeadersFromMetadata(data?.metadata?.request_headers),
    [data?.metadata],
  )
  const responseHeaders = useMemo(
    () => parseHeadersFromMetadata(data?.metadata?.response_headers),
    [data?.metadata],
  )
  const metadataRest = useMemo(
    () => metadataWithoutHeaders(data?.metadata ?? null),
    [data?.metadata],
  )

  const byokListEntry = useMemo(() => {
    if (auditUpstream.profileId == null) return undefined
    const row = byokRes?.data?.find((p) => p.id === auditUpstream.profileId)
    if (!row) return undefined
    return { name: row.name, revoked: row.revoked }
  }, [auditUpstream.profileId, byokRes?.data])

  if (!decoded) {
    return <p className="text-sm text-muted-foreground">无效的 request_id</p>
  }

  return (
    <section className="space-y-6">
      <div className="flex flex-wrap items-center gap-3">
        <Button variant="outline" size="sm" asChild>
          <Link to="/logs">← 返回列表</Link>
        </Button>
        <h1 className="text-xl font-semibold">日志详情</h1>
      </div>

      {isLoading && <p className="text-sm text-muted-foreground">加载中…</p>}
      {isError && (
        <p className="text-sm text-red-600" role="alert">
          无法加载该条记录（不存在或无权访问）。
        </p>
      )}

      {data && (
        <>
          {(() => {
            const meta = data.metadata
            const streamAborted =
              meta !== null &&
              typeof meta === 'object' &&
              !Array.isArray(meta) &&
              meta['stream_aborted'] === true
            return streamAborted ? (
              <p className="text-sm text-amber-800 dark:text-amber-200" role="status">
                流式连接曾中断，响应文件为已接收部分（非业务截断）。
              </p>
            ) : null
          })()}
          <Card className="space-y-3 p-4">
            <dl className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              <div>
                <dt className="text-xs text-muted-foreground">request_id</dt>
                <dd className="mt-0.5 break-all font-mono text-sm">{data.request_id}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">时间</dt>
                <dd className="mt-0.5 text-sm">{formatTime(data.created_at)}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">HTTP 状态</dt>
                <dd className="mt-0.5 text-sm">{data.status_code ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">模型</dt>
                <dd className="mt-0.5 text-sm">{data.model ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">应用 (app_id)</dt>
                <dd className="mt-0.5 text-sm">{data.app_id ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">会话 (thread_id)</dt>
                <dd className="mt-0.5 break-all font-mono text-sm">{data.thread_id ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">类型</dt>
                <dd className="mt-0.5 text-sm">{data.request_type ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">调用上游</dt>
                <dd className="mt-0.5 text-sm">
                  {auditUpstream.isByok === true
                    ? 'BYOK（自有上游）'
                    : auditUpstream.isByok === false
                      ? 'ModelGate 平台上游'
                      : '—'}
                </dd>
              </div>
              {auditUpstream.profileId != null ? (
                <div>
                  <dt className="text-xs text-muted-foreground">BYOK 配置</dt>
                  <dd className="mt-0.5 text-sm">
                    <Link
                      to={`/byok-profiles/${auditUpstream.profileId}`}
                      className="text-primary hover:underline"
                    >
                      {byokListEntry
                        ? `「${byokListEntry.name}」(#${auditUpstream.profileId})`
                        : `配置 #${auditUpstream.profileId}`}
                    </Link>
                    {byokListEntry?.revoked ? (
                      <span className="ml-1 text-xs text-muted-foreground">
                        （已吊销）
                      </span>
                    ) : null}
                  </dd>
                </div>
              ) : null}
              <div>
                <dt className="text-xs text-muted-foreground">用户 / 令牌</dt>
                <dd className="mt-0.5 font-mono text-sm">
                  {data.user_id ?? '—'} / {data.token_id ?? '—'}
                </dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">渠道</dt>
                <dd className="mt-0.5 text-sm">{data.channel_id ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">输入 Tokens</dt>
                <dd className="mt-0.5 font-mono text-sm">{data.prompt_tokens ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">输出 Tokens</dt>
                <dd className="mt-0.5 font-mono text-sm">
                  {data.completion_tokens ?? '—'}
                </dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">合计 Tokens</dt>
                <dd className="mt-0.5 font-mono text-sm">{data.total_tokens ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Finish 原因</dt>
                <dd className="mt-0.5 font-mono text-sm">{data.finish_reason ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">耗时 (ms)</dt>
                <dd className="mt-0.5 font-mono text-sm">{data.latency_ms ?? '—'}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">成本</dt>
                <dd className="mt-0.5 font-mono text-sm">
                  {data.cost != null ? formatCostUsd(data.cost) : '—'}
                </dd>
              </div>
            </dl>
            {data.error_message && (
              <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm">
                <p className="font-medium text-amber-900 dark:text-amber-200">错误信息</p>
                <p className="mt-1 whitespace-pre-wrap">{data.error_message}</p>
              </div>
            )}
          </Card>

          <div className="grid gap-4 lg:grid-cols-2">
            <AuditHeadersPanel title="请求头" headers={requestHeaders} />
            <AuditHeadersPanel title="响应头（上游）" headers={responseHeaders} />
          </div>

          {usage && (
            <UsageDetailCard usage={usage} auditUpstream={auditUpstream} />
          )}

          <AuditBodyPanel
            title="请求体"
            requestId={decoded}
            part="request"
            hasPath={Boolean(data.request_body_path)}
            emptyHint="未记录请求体文件路径（写入失败或未配置审计目录时可能出现）。"
          />
          <AuditBodyPanel
            title="响应体"
            requestId={decoded}
            part="response"
            hasPath={Boolean(data.response_body_path)}
            streamSseNote={
              data.metadata != null &&
              typeof data.metadata === 'object' &&
              !Array.isArray(data.metadata) &&
              data.metadata['response_body_format'] === 'text/event-stream'
            }
            emptyHint={
              data.metadata != null &&
              typeof data.metadata === 'object' &&
              !Array.isArray(data.metadata) &&
              data.metadata['stream'] === true
                ? '若为流式请求，持久化结束后将显示路径与正文；也可稍后刷新页面。'
                : '无已保存的响应体（例如上游失败时尚未写入）。'
            }
          />

          {metadataRest && Object.keys(metadataRest).length > 0 && (
            <Card className="p-4">
              <h2 className="text-sm font-medium">metadata</h2>
              <pre className="mt-2 max-h-64 overflow-auto rounded bg-muted/50 p-3 font-mono text-xs">
                {JSON.stringify(metadataRest, null, 2)}
              </pre>
            </Card>
          )}
        </>
      )}
    </section>
  )
}
