import { useEffect, useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import {
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

  const statCards = useMemo(() => {
    if (!data) {
      return [
        { title: '近 24h 请求', value: '—', hint: '' },
        { title: '成功率', value: '—', hint: '' },
        { title: '总 Token', value: '—', hint: '' },
        { title: '总成本 (USD)', value: '—', hint: '' },
      ]
    }
    const s = data.summary
    const successPct =
      s.total_requests > 0
        ? ((s.success_requests / s.total_requests) * 100).toFixed(1)
        : '—'
    return [
      {
        title: '近 24h 请求',
        value: s.total_requests.toLocaleString(),
        hint: '',
      },
      {
        title: '成功率',
        value: successPct === '—' ? '—' : `${successPct}%`,
        hint: '',
      },
      {
        title: '总 Token',
        value: s.total_tokens.toLocaleString(),
        hint: '',
      },
      {
        title: '总成本 (USD)',
        value:
          s.total_cost > 0
            ? formatCostUsd(s.total_cost)
            : s.total_cost === 0
              ? '$0'
              : '—',
        hint: '',
      },
    ]
  }, [data])

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
        {statCards.map((item) => (
          <article
            key={item.title}
            className="rounded-lg border border-border bg-card p-4 shadow-sm"
          >
            <p className="text-sm text-muted-foreground">{item.title}</p>
            <p className="mt-2 text-2xl font-semibold">{item.value}</p>
            {item.hint ? (
              <p className="mt-1 text-xs text-muted-foreground">{item.hint}</p>
            ) : null}
          </article>
        ))}
      </div>

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
