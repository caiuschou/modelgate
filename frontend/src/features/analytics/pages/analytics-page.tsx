import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import {
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/shared/empty-state'
import { useAnalytics } from '@/features/analytics/hooks/use-analytics'

function unixNow(): number {
  return Math.floor(Date.now() / 1000)
}

function formatBucketLabel(ts: number, bucketSec: number): string {
  const d = new Date(ts * 1000)
  if (bucketSec <= 3600) {
    return d.toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }
  if (bucketSec <= 86400) {
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  }
  return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' })
}

export function AnalyticsPage() {
  const [preset, setPreset] = useState<'24h' | '7d' | '30d'>('7d')
  const [modelDraft, setModelDraft] = useState('')
  const [modelFilter, setModelFilter] = useState('')
  const [appIdDraft, setAppIdDraft] = useState('')
  const [appIdFilter, setAppIdFilter] = useState('')
  const [threadIdDraft, setThreadIdDraft] = useState('')
  const [threadIdFilter, setThreadIdFilter] = useState('')

  const range = useMemo(() => {
    const end = unixNow()
    const sec =
      preset === '24h' ? 24 * 3600 : preset === '7d' ? 7 * 24 * 3600 : 30 * 24 * 3600
    return { start: end - sec, end }
  }, [preset])

  const query = useMemo(
    () => ({
      start_time: range.start,
      end_time: range.end,
      ...(modelFilter.trim() ? { model: modelFilter.trim() } : {}),
      ...(appIdFilter.trim() ? { app_id: appIdFilter.trim() } : {}),
      ...(threadIdFilter.trim() ? { thread_id: threadIdFilter.trim() } : {}),
    }),
    [range.start, range.end, modelFilter, appIdFilter, threadIdFilter],
  )

  const { data, isLoading, isError, error, refetch, isFetching } = useAnalytics(query)

  const chartData = useMemo(() => {
    if (!data?.series.length) return []
    return data.series.map((row) => ({
      ...row,
      label: formatBucketLabel(row.bucket_start, data.bucket_seconds),
    }))
  }, [data])

  const successRate =
    data && data.summary.total_requests > 0
      ? (
          (data.summary.success_requests / data.summary.total_requests) *
          100
        ).toFixed(1)
      : '—'

  const logsHref = useMemo(() => {
    const p = new URLSearchParams()
    p.set('start_time', String(range.start))
    p.set('end_time', String(range.end))
    if (modelFilter.trim()) p.set('model', modelFilter.trim())
    if (appIdFilter.trim()) p.set('app_id', appIdFilter.trim())
    if (threadIdFilter.trim()) p.set('thread_id', threadIdFilter.trim())
    return `/logs?${p.toString()}`
  }, [range.start, range.end, modelFilter, appIdFilter, threadIdFilter])

  return (
    <section>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">统计分析</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            基于审计日志聚合，范围最长 366 天。数据与
            <Link to={logsHref} className="ml-1 underline underline-offset-2">
              日志中心
            </Link>
            一致。
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => refetch()}
          disabled={isFetching}
        >
          {isFetching ? '刷新中…' : '刷新'}
        </Button>
      </div>

      <div className="mt-4 flex flex-wrap items-end gap-3">
        <div>
          <p className="mb-1 text-xs text-muted-foreground">时间范围</p>
          <div className="flex flex-wrap gap-2">
            {(
              [
                ['24h', '24h' as const],
                ['近 7 天', '7d' as const],
                ['近 30 天', '30d' as const],
              ] as const
            ).map(([label, key]) => (
              <Button
                key={key}
                variant={preset === key ? 'default' : 'outline'}
                size="sm"
                onClick={() => setPreset(key)}
              >
                {label}
              </Button>
            ))}
          </div>
        </div>
        <div className="flex min-w-[200px] flex-1 flex-col gap-1 sm:max-w-sm">
          <span className="text-xs text-muted-foreground">模型（精确匹配）</span>
          <div className="flex gap-2">
            <Input
              value={modelDraft}
              onChange={(e) => setModelDraft(e.target.value)}
              placeholder="留空表示全部"
              className="h-9"
            />
            <Button
              type="button"
              size="sm"
              variant="secondary"
              onClick={() => setModelFilter(modelDraft)}
            >
              应用
            </Button>
          </div>
        </div>
        <div className="flex min-w-[200px] flex-1 flex-col gap-1 sm:max-w-sm">
          <span className="text-xs text-muted-foreground">应用 app_id（精确匹配）</span>
          <div className="flex gap-2">
            <Input
              value={appIdDraft}
              onChange={(e) => setAppIdDraft(e.target.value)}
              placeholder="留空表示全部"
              className="h-9"
            />
            <Button
              type="button"
              size="sm"
              variant="secondary"
              onClick={() => setAppIdFilter(appIdDraft)}
            >
              应用
            </Button>
          </div>
        </div>
        <div className="flex min-w-[200px] flex-1 flex-col gap-1 sm:max-w-sm">
          <span className="text-xs text-muted-foreground">会话 thread_id（精确匹配）</span>
          <div className="flex gap-2">
            <Input
              value={threadIdDraft}
              onChange={(e) => setThreadIdDraft(e.target.value)}
              placeholder="留空表示全部"
              className="h-9"
            />
            <Button
              type="button"
              size="sm"
              variant="secondary"
              onClick={() => setThreadIdFilter(threadIdDraft)}
            >
              应用
            </Button>
          </div>
        </div>
      </div>

      {isError && (
        <p className="mt-4 text-sm text-destructive" role="alert">
          {(error as Error)?.message ?? '加载统计数据失败'}
        </p>
      )}

      {isLoading && !data ? (
        <p className="mt-6 text-sm text-muted-foreground">加载中…</p>
      ) : data ? (
        <>
          <div className="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
            <Card className="p-4">
              <p className="text-sm text-muted-foreground">总请求</p>
              <p className="mt-2 text-2xl font-semibold tabular-nums">
                {data.summary.total_requests.toLocaleString()}
              </p>
            </Card>
            <Card className="p-4">
              <p className="text-sm text-muted-foreground">成功率（2xx）</p>
              <p className="mt-2 text-2xl font-semibold tabular-nums">{successRate}%</p>
            </Card>
            <Card className="p-4">
              <p className="text-sm text-muted-foreground">总 Token</p>
              <p className="mt-2 text-2xl font-semibold tabular-nums">
                {data.summary.total_tokens.toLocaleString()}
              </p>
            </Card>
            <Card className="p-4">
              <p className="text-sm text-muted-foreground">成本合计</p>
              <p className="mt-2 text-2xl font-semibold tabular-nums">
                {data.summary.total_cost > 0
                  ? data.summary.total_cost.toLocaleString(undefined, {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 6,
                    })
                  : '—'}
              </p>
            </Card>
            <Card className="p-4">
              <p className="text-sm text-muted-foreground">平均延迟</p>
              <p className="mt-2 text-2xl font-semibold tabular-nums">
                {data.summary.avg_latency_ms != null
                  ? `${Math.round(data.summary.avg_latency_ms)} ms`
                  : '—'}
              </p>
            </Card>
          </div>

          <div className="mt-2 text-xs text-muted-foreground">
            桶粒度：每{' '}
            {data.bucket_seconds >= 86400
              ? `${data.bucket_seconds / 86400} 天`
              : `${data.bucket_seconds / 3600} 小时`}
          </div>

          <Card className="mt-6 p-4">
            <h2 className="text-sm font-medium">请求与 Token 趋势</h2>
            {chartData.length === 0 ? (
              <div className="mt-8">
                <EmptyState
                  title="暂无序列数据"
                  description="当前时间范围内没有审计记录，或筛选过窄。"
                />
              </div>
            ) : (
              <div className="mt-4 h-72 w-full min-w-0">
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={chartData} margin={{ top: 8, right: 8, left: 0, bottom: 0 }}>
                    <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                    <XAxis dataKey="label" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                    <YAxis yAxisId="left" tick={{ fontSize: 11 }} allowDecimals={false} />
                    <YAxis
                      yAxisId="right"
                      orientation="right"
                      tick={{ fontSize: 11 }}
                      allowDecimals={false}
                    />
                    <Tooltip
                      contentStyle={{
                        background: 'hsl(var(--card))',
                        border: '1px solid hsl(var(--border))',
                        borderRadius: 8,
                      }}
                    />
                    <Legend />
                    <Line
                      yAxisId="left"
                      type="monotone"
                      dataKey="request_count"
                      name="请求数"
                      stroke="hsl(var(--primary))"
                      dot={false}
                      strokeWidth={2}
                    />
                    <Line
                      yAxisId="right"
                      type="monotone"
                      dataKey="total_tokens"
                      name="Token"
                      stroke="hsl(142 76% 36%)"
                      dot={false}
                      strokeWidth={2}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            )}
          </Card>

          <Card className="mt-6 p-4">
            <h2 className="text-sm font-medium">按模型分布（Top 30）</h2>
            {data.by_model.length === 0 ? (
              <div className="mt-8">
                <EmptyState title="暂无模型分布" description="该范围内无模型数据。" />
              </div>
            ) : (
              <div className="mt-4 h-[min(28rem,60vh)] w-full min-w-0">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart
                    data={[...data.by_model].reverse()}
                    layout="vertical"
                    margin={{ top: 8, right: 16, left: 8, bottom: 8 }}
                  >
                    <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                    <XAxis type="number" tick={{ fontSize: 11 }} allowDecimals={false} />
                    <YAxis
                      type="category"
                      dataKey="model"
                      width={120}
                      tick={{ fontSize: 11 }}
                    />
                    <Tooltip
                      contentStyle={{
                        background: 'hsl(var(--card))',
                        border: '1px solid hsl(var(--border))',
                        borderRadius: 8,
                      }}
                    />
                    <Legend />
                    <Bar dataKey="request_count" name="请求数" fill="hsl(var(--primary))" />
                    <Bar dataKey="total_tokens" name="Token" fill="hsl(142 76% 36%)" />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            )}
          </Card>
        </>
      ) : null}
    </section>
  )
}
