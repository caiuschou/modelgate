import { Link, useParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { useAuditLogDetail } from '@/features/logs/hooks/use-logs'

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

export function LogDetailPage() {
  const { requestId } = useParams<{ requestId: string }>()
  const decoded = requestId ? decodeURIComponent(requestId) : ''
  const { data, isLoading, isError } = useAuditLogDetail(decoded || undefined)

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
                <dt className="text-xs text-muted-foreground">类型</dt>
                <dd className="mt-0.5 text-sm">{data.request_type ?? '—'}</dd>
              </div>
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
                <dd className="mt-0.5 font-mono text-sm">{data.cost ?? '—'}</dd>
              </div>
            </dl>
            {data.error_message && (
              <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm">
                <p className="font-medium text-amber-900 dark:text-amber-200">错误信息</p>
                <p className="mt-1 whitespace-pre-wrap">{data.error_message}</p>
              </div>
            )}
          </Card>

          <Card className="space-y-3 p-4">
            <h2 className="text-sm font-medium">存储路径</h2>
            <p className="text-xs text-muted-foreground">
              请求/响应体保存在网关审计目录，以下为服务端记录的路径引用。
            </p>
            <dl className="space-y-2 text-sm">
              <div>
                <dt className="text-muted-foreground">请求体</dt>
                <dd className="break-all font-mono text-xs">
                  {data.request_body_path ?? '—'}
                </dd>
              </div>
              <div>
                <dt className="text-muted-foreground">响应体</dt>
                <dd className="break-all font-mono text-xs">
                  {data.response_body_path ?? '—'}
                </dd>
              </div>
            </dl>
          </Card>

          {data.metadata && Object.keys(data.metadata).length > 0 && (
            <Card className="p-4">
              <h2 className="text-sm font-medium">metadata</h2>
              <pre className="mt-2 max-h-64 overflow-auto rounded bg-muted/50 p-3 font-mono text-xs">
                {JSON.stringify(data.metadata, null, 2)}
              </pre>
            </Card>
          )}
        </>
      )}
    </section>
  )
}
