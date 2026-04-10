import { useEffect, useId, useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { Card } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import {
  analyticsRangeFor24HourlyBars,
  hourBucketStarts24,
} from '@/features/dashboard/dashboard-analytics-range'
import { useAnalytics } from '@/features/analytics/hooks/use-analytics'
import { formatCostUsd } from '@/lib/format-cost'
import { useTeamStore } from '@/stores/team-store'

function unixNow(): number {
  return Math.floor(Date.now() / 1000)
}

function formatHourLabel(ts: number): string {
  return new Date(ts * 1000).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

type SeriesRow = {
  bucket_start: number
  request_count: number
  total_tokens: number
  total_cost: number
  prompt_tokens: number
  completion_tokens: number
  cached_prompt_tokens: number
}

/** Always 24 rows; missing API buckets filled with zeros. */
function mergeSeriesTo24HourlyBars(
  endUnix: number,
  series: SeriesRow[] | undefined,
): Array<SeriesRow & { label: string }> {
  const starts = hourBucketStarts24(endUnix)
  const byStart = new Map<number, SeriesRow>()
  for (const row of series ?? []) {
    byStart.set(row.bucket_start, row)
  }
  return starts.map((bucket_start) => {
    const row = byStart.get(bucket_start)
    return {
      bucket_start,
      request_count: row?.request_count ?? 0,
      total_tokens: row?.total_tokens ?? 0,
      total_cost: row?.total_cost ?? 0,
      prompt_tokens: row?.prompt_tokens ?? 0,
      completion_tokens: row?.completion_tokens ?? 0,
      cached_prompt_tokens: row?.cached_prompt_tokens ?? 0,
      label: formatHourLabel(bucket_start),
    }
  })
}

/** Segments for a single stacked column (prompt cache / prompt other / completion). */
type TokenStackRow = SeriesRow & {
  label: string
  token_prompt_cache: number
  token_prompt_other: number
  token_completion: number
}

function toTokenStackRows(rows: Array<SeriesRow & { label: string }>): TokenStackRow[] {
  return rows.map((r) => {
    const pt = r.prompt_tokens
    const cpt = Math.min(Math.max(0, r.cached_prompt_tokens), pt)
    const pn = Math.max(0, pt - cpt)
    return {
      ...r,
      token_prompt_cache: cpt,
      token_prompt_other: pn,
      token_completion: r.completion_tokens,
    }
  })
}

function rollupTokenSegments(rows: TokenStackRow[]) {
  return rows.reduce(
    (acc, r) => ({
      cache: acc.cache + r.token_prompt_cache,
      promptOther: acc.promptOther + r.token_prompt_other,
      completion: acc.completion + r.token_completion,
    }),
    { cache: 0, promptOther: 0, completion: 0 },
  )
}

/** Match stacked bar chart colors below. */
const TOKEN_COL = {
  cache: 'hsl(38 92% 50%)',
  prompt: 'hsl(217 91% 55%)',
  completion: 'hsl(142 76% 36%)',
} as const

function MiniSparkline({
  values,
  stroke,
  fillStopOpacity = 0.22,
  className,
}: {
  values: number[]
  stroke: string
  fillStopOpacity?: number
  className?: string
}) {
  const gid = useId().replace(/:/g, '')
  const data = values.map((v, i) => ({ i, v }))
  const hasActivity = values.some((v) => v > 0)
  if (!hasActivity) {
    return (
      <div
        className={cn(
          'flex h-14 items-end justify-between gap-px rounded-md bg-muted/40 px-1 pt-2',
          className,
        )}
        aria-hidden
      >
        {Array.from({ length: Math.min(24, values.length || 24) }, (_, i) => (
          <div key={i} className="h-3 w-full max-w-[8px] rounded-sm bg-muted-foreground/10" />
        ))}
      </div>
    )
  }
  return (
    <div className={cn('h-14 w-full min-w-0', className)}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={data} margin={{ top: 4, right: 2, left: 2, bottom: 0 }}>
          <defs>
            <linearGradient id={`sf-${gid}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={stroke} stopOpacity={fillStopOpacity} />
              <stop offset="100%" stopColor={stroke} stopOpacity={0} />
            </linearGradient>
          </defs>
          <Area
            type="monotone"
            dataKey="v"
            stroke={stroke}
            strokeWidth={1.5}
            fill={`url(#sf-${gid})`}
            isAnimationActive={false}
            dot={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}

function SuccessRateBar({ pct }: { pct: number | null }) {
  const width = pct == null ? 0 : Math.min(100, Math.max(0, pct))
  return (
    <div
      className="mt-3"
      role="img"
      aria-label={pct == null ? '暂无请求，无成功率' : `成功率约 ${pct.toFixed(1)}%`}
    >
      <div className="h-2.5 w-full overflow-hidden rounded-full bg-muted">
        <div
          className="h-full rounded-full bg-emerald-500/90 transition-[width] dark:bg-emerald-400/85"
          style={{ width: `${width}%` }}
        />
      </div>
    </div>
  )
}

function TokenCompositionVisual({
  seg,
  total,
  avgPerReq,
}: {
  seg: { cache: number; promptOther: number; completion: number }
  total: number
  avgPerReq: number | null
}) {
  const safe = total > 0 ? total : 1
  const wCache = (seg.cache / safe) * 100
  const wPrompt = (seg.promptOther / safe) * 100
  const wComp = (seg.completion / safe) * 100
  const parts = [
    { key: 'a', w: wCache, color: TOKEN_COL.cache, n: seg.cache },
    { key: 'b', w: wPrompt, color: TOKEN_COL.prompt, n: seg.promptOther },
    { key: 'c', w: wComp, color: TOKEN_COL.completion, n: seg.completion },
  ].filter((p) => p.n > 0)

  if (total <= 0) {
    return (
      <p className="mt-3 text-xs text-muted-foreground">该时段尚无 Token 用量</p>
    )
  }

  return (
    <div className="mt-3 space-y-3.5">
      <div
        className="flex h-2.5 w-full overflow-hidden rounded-full bg-muted/70"
        role="img"
        aria-label="Token 组成占比"
      >
        {parts.map((p, i) => (
          <div
            key={p.key}
            className={cn(
              'h-full min-w-[2px]',
              i === 0 && 'rounded-l-full',
              i === parts.length - 1 && 'rounded-r-full',
            )}
            style={{ width: `${p.w}%`, backgroundColor: p.color }}
            title={`${p.n.toLocaleString()} (${p.w.toFixed(0)}%)`}
          />
        ))}
      </div>
      <ul className="flex flex-col gap-3 text-xs leading-relaxed">
        <li className="flex items-start justify-between gap-4">
          <span className="flex min-w-0 items-start gap-2 pt-0.5">
            <span
              className="mt-0.5 h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: TOKEN_COL.cache }}
            />
            <span className="text-muted-foreground">Prompt 缓存</span>
          </span>
          <span className="shrink-0 text-right">
            <span className="block font-medium tabular-nums text-foreground">
              {seg.cache.toLocaleString()}
            </span>
            <span className="mt-0.5 block text-[0.7rem] text-muted-foreground tabular-nums">
              占 {wCache.toFixed(0)}%
            </span>
          </span>
        </li>
        <li className="flex items-start justify-between gap-4">
          <span className="flex min-w-0 items-start gap-2 pt-0.5">
            <span
              className="mt-0.5 h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: TOKEN_COL.prompt }}
            />
            <span className="text-muted-foreground">Prompt 非缓存</span>
          </span>
          <span className="shrink-0 text-right">
            <span className="block font-medium tabular-nums text-foreground">
              {seg.promptOther.toLocaleString()}
            </span>
            <span className="mt-0.5 block text-[0.7rem] text-muted-foreground tabular-nums">
              占 {wPrompt.toFixed(0)}%
            </span>
          </span>
        </li>
        <li className="flex items-start justify-between gap-4">
          <span className="flex min-w-0 items-start gap-2 pt-0.5">
            <span
              className="mt-0.5 h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: TOKEN_COL.completion }}
            />
            <span className="text-muted-foreground">Completion</span>
          </span>
          <span className="shrink-0 text-right">
            <span className="block font-medium tabular-nums text-foreground">
              {seg.completion.toLocaleString()}
            </span>
            <span className="mt-0.5 block text-[0.7rem] text-muted-foreground tabular-nums">
              占 {wComp.toFixed(0)}%
            </span>
          </span>
        </li>
      </ul>
      {avgPerReq != null ? (
        <p className="border-t border-border/60 pt-2.5 text-xs leading-relaxed text-muted-foreground">
          单次请求约{' '}
          <span className="font-medium tabular-nums text-foreground">{avgPerReq.toLocaleString()}</span>{' '}
          Token
        </p>
      ) : null}
    </div>
  )
}

function RequestsVisual({ spark, latencyMs }: { spark: number[]; latencyMs: number | null }) {
  return (
    <div className="mt-3">
      <MiniSparkline values={spark} stroke="hsl(217 91% 55%)" />
      {latencyMs != null ? (
        <p className="mt-2 text-xs text-muted-foreground">
          平均延迟{' '}
          <span className="font-medium tabular-nums text-foreground">
            {Math.round(latencyMs).toLocaleString()}
          </span>{' '}
          ms
        </p>
      ) : null}
    </div>
  )
}

function CostExtrasVisual({
  cpm,
  sparklineValues,
}: {
  cpm: number | null
  sparklineValues: number[]
}) {
  return (
    <div className="mt-3 space-y-2">
      <MiniSparkline values={sparklineValues} stroke="hsl(var(--primary))" />
      {cpm != null ? (
        <p className="text-xs text-muted-foreground">
          约合{' '}
          <span className="font-medium tabular-nums text-foreground">{formatCostUsd(cpm)}</span> / 百万
          Token
        </p>
      ) : null}
    </div>
  )
}

export function DashboardPage() {
  const [windowTick, setWindowTick] = useState(0)
  const teamId = useTeamStore((s) => s.currentTeamId)
  useEffect(() => {
    const id = window.setInterval(() => setWindowTick((t) => t + 1), 60_000)
    return () => window.clearInterval(id)
  }, [])

  const query = useMemo(() => {
    void windowTick
    const end = unixNow()
    const { start, end: endIncl } = analyticsRangeFor24HourlyBars(end)
    return {
      start_time: start,
      end_time: endIncl,
      // Default console is "personal" (no X-Team-Id); team-key traffic has team_id set and was invisible.
      ...(teamId == null ? { combined: true as const } : {}),
    }
  }, [windowTick, teamId])

  const { data, isLoading, isError } = useAnalytics(query)

  const costChart = useMemo(() => {
    if (!data) return []
    return mergeSeriesTo24HourlyBars(query.end_time, data.series)
  }, [data, query.end_time])

  const tokenChart = useMemo(() => {
    if (!data) return []
    return toTokenStackRows(mergeSeriesTo24HourlyBars(query.end_time, data.series))
  }, [data, query.end_time])

  const metrics = useMemo(() => {
    if (!data) return null
    const s = data.summary
    const seg = rollupTokenSegments(tokenChart)
    const successPct =
      s.total_requests > 0 ? (s.success_requests / s.total_requests) * 100 : null
    const avgPerReq =
      s.total_requests > 0 && s.total_tokens > 0
        ? Math.round(s.total_tokens / s.total_requests)
        : null
    const cpm =
      s.total_tokens > 0 && s.total_cost >= 0 ? (s.total_cost / s.total_tokens) * 1e6 : null
    const requestSpark = costChart.map((r) => r.request_count)
    const costSpark = costChart.map((r) => r.total_cost)
    const avgLatencyMs =
      s.total_requests > 0 && s.avg_latency_ms != null && Number.isFinite(s.avg_latency_ms)
        ? s.avg_latency_ms
        : null
    const costValue =
      s.total_cost > 0
        ? formatCostUsd(s.total_cost)
        : s.total_cost === 0
          ? '$0'
          : '—'
    return {
      total_requests: s.total_requests,
      successPct,
      successDisplay: successPct == null ? '—' : `${successPct.toFixed(1)}%`,
      totalTokens: s.total_tokens,
      costValue,
      seg,
      avgPerReq,
      cpm,
      avgLatencyMs,
      requestSpark,
      costSpark,
    }
  }, [data, costChart, tokenChart])

  const topModels = useMemo(() => {
    if (!data?.by_model?.length) return []
    return [...data.by_model]
      .sort((a, b) => b.total_tokens - a.total_tokens)
      .slice(0, 3)
  }, [data])

  const totalTokensForShare = data?.summary.total_tokens ?? 0

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">仪表盘</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          近 24 个整点小时（含当前时段）消费与用量；详情见{' '}
          <Link to="/analytics" className="underline underline-offset-2">
            统计分析
          </Link>
          。
        </p>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <article className="rounded-lg border border-border bg-card p-4 shadow-sm">
          <p className="text-sm text-muted-foreground">近 24h 请求</p>
          <p className="mt-2 text-2xl font-semibold tabular-nums">
            {metrics ? metrics.total_requests.toLocaleString() : '—'}
          </p>
          {metrics ? (
            <RequestsVisual spark={metrics.requestSpark} latencyMs={metrics.avgLatencyMs} />
          ) : (
            <div className="mt-3 h-14 rounded-md bg-muted/40" aria-hidden />
          )}
        </article>

        <article className="rounded-lg border border-border bg-card p-4 shadow-sm">
          <p className="text-sm text-muted-foreground">成功率</p>
          <p className="mt-2 text-2xl font-semibold tabular-nums">
            {metrics ? metrics.successDisplay : '—'}
          </p>
          {metrics ? <SuccessRateBar pct={metrics.successPct} /> : <div className="mt-3 h-2.5 rounded-full bg-muted/40" aria-hidden />}
        </article>

        <article className="rounded-lg border border-border bg-card p-4 shadow-sm">
          <p className="text-sm text-muted-foreground">近 24h Token 合计</p>
          <p className="mt-2 text-2xl font-semibold tabular-nums">
            {metrics ? metrics.totalTokens.toLocaleString() : '—'}
          </p>
          {metrics ? (
            <TokenCompositionVisual
              seg={metrics.seg}
              total={metrics.totalTokens}
              avgPerReq={metrics.avgPerReq}
            />
          ) : (
            <div className="mt-3 h-2.5 rounded-full bg-muted/40" aria-hidden />
          )}
        </article>

        <article className="rounded-lg border border-border bg-card p-4 shadow-sm">
          <p className="text-sm text-muted-foreground">近 24h 成本 (USD)</p>
          <p className="mt-2 text-2xl font-semibold tabular-nums">
            {metrics ? metrics.costValue : '—'}
          </p>
          {metrics ? (
            <CostExtrasVisual cpm={metrics.cpm} sparklineValues={metrics.costSpark} />
          ) : (
            <div className="mt-3 h-14 rounded-md bg-muted/40" aria-hidden />
          )}
        </article>
      </div>

      {topModels.length > 0 ? (
        <Card className="p-4">
          <h2 className="text-sm font-medium">近 24h · 主要模型（按 Token）</h2>
          <ul className="mt-3 space-y-4 text-sm">
            {topModels.map((m) => {
              const sharePct =
                totalTokensForShare > 0 ? (m.total_tokens / totalTokensForShare) * 100 : 0
              return (
                <li key={m.model}>
                  <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
                    <span className="min-w-0 break-all font-mono text-xs">{m.model}</span>
                    <span className="shrink-0 tabular-nums text-muted-foreground">
                      {m.total_tokens.toLocaleString()} Token · {m.request_count.toLocaleString()} 次
                      {totalTokensForShare > 0 ? (
                        <span className="ml-2 text-foreground">({sharePct.toFixed(1)}%)</span>
                      ) : null}
                    </span>
                  </div>
                  <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-muted">
                    <div
                      className="h-full rounded-full bg-primary/85 transition-[width]"
                      style={{ width: `${Math.min(100, sharePct)}%` }}
                    />
                  </div>
                </li>
              )
            })}
          </ul>
        </Card>
      ) : null}

      <Card className="p-4">
        <h2 className="text-sm font-medium">近 24 小时 · 按小时成本 (USD)</h2>
        <p className="mt-1 text-xs text-muted-foreground">
          固定 24 个整点小时；该小时无请求则显示为 0。金额为审计中的上游 cost。
        </p>
        {isError && (
          <p className="mt-4 text-sm text-red-600" role="alert">
            无法加载统计数据。
          </p>
        )}
        {isLoading && !data && (
          <p className="mt-4 text-sm text-muted-foreground">加载中…</p>
        )}
        {!isLoading && data && (
          <div className="mt-4 h-[min(22rem,50vh)] w-full min-w-0">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={costChart} margin={{ top: 8, right: 8, left: 8, bottom: 8 }}>
                <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                <XAxis
                  dataKey="label"
                  tick={{ fontSize: 10 }}
                  interval="preserveStartEnd"
                  minTickGap={24}
                />
                <YAxis
                  tick={{ fontSize: 11 }}
                  tickFormatter={(v) =>
                    typeof v === 'number' && Math.abs(v) < 0.01 && v !== 0
                      ? v.toExponential(0)
                      : String(v)
                  }
                />
                <Tooltip
                  contentStyle={{
                    background: 'hsl(var(--card))',
                    border: '1px solid hsl(var(--border))',
                    borderRadius: 8,
                  }}
                  formatter={(value) =>
                    typeof value === 'number' ? formatCostUsd(value) : String(value ?? '—')
                  }
                />
                <Bar
                  dataKey="total_cost"
                  name="成本"
                  fill="hsl(var(--primary))"
                  radius={[4, 4, 0, 0]}
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}
      </Card>

      <Card className="p-4">
        <h2 className="text-sm font-medium">近 24 小时 · Token（单柱堆叠）</h2>
        <p className="mt-1 text-xs text-muted-foreground">
          自下而上：Prompt 缓存命中 → Prompt 非缓存 → Completion。无缓存字段时全部为「非缓存」。
        </p>
        {isError && (
          <p className="mt-4 text-sm text-red-600" role="alert">
            无法加载统计数据。
          </p>
        )}
        {isLoading && !data && (
          <p className="mt-4 text-sm text-muted-foreground">加载中…</p>
        )}
        {!isLoading && data && (
          <div className="mt-4 h-[min(22rem,50vh)] w-full min-w-0">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={tokenChart} margin={{ top: 8, right: 8, left: 8, bottom: 8 }}>
                <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                <XAxis
                  dataKey="label"
                  tick={{ fontSize: 10 }}
                  interval="preserveStartEnd"
                  minTickGap={24}
                />
                <YAxis tick={{ fontSize: 11 }} allowDecimals={false} />
                <Tooltip
                  contentStyle={{
                    background: 'hsl(var(--card))',
                    border: '1px solid hsl(var(--border))',
                    borderRadius: 8,
                  }}
                  formatter={(value) =>
                    typeof value === 'number' ? value.toLocaleString() : String(value ?? '—')
                  }
                />
                <Legend wrapperStyle={{ fontSize: 12 }} />
                <Bar
                  dataKey="token_prompt_cache"
                  name="Prompt（缓存）"
                  stackId="tok"
                  fill="hsl(38 92% 50%)"
                />
                <Bar
                  dataKey="token_prompt_other"
                  name="Prompt（非缓存）"
                  stackId="tok"
                  fill="hsl(217 91% 55%)"
                />
                <Bar
                  dataKey="token_completion"
                  name="Completion"
                  stackId="tok"
                  fill="hsl(142 76% 36%)"
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}
      </Card>
    </section>
  )
}
