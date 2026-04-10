import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { ChevronDown, RefreshCw, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/shared/empty-state'
import { LogDateField } from '@/features/logs/components/log-date-field'
import {
  downloadExportFile,
  useAuditLogList,
  useExportAuditLogs,
} from '@/features/logs/hooks/use-logs'
import { formatCostUsd } from '@/lib/format-cost'
import { cn } from '@/lib/utils'

const PAGE_SIZE = 20

/** Ensures the refresh icon spins long enough to perceive on fast LAN responses. */
const REFRESH_SPIN_MIN_MS = 280

const ADVANCED_PARAM_KEYS = [
  'app_id',
  'finish_reason',
  'status_code',
  'token_id',
] as const

type AppliedFilterKey =
  | 'keyword'
  | 'model'
  | 'app_id'
  | 'finish_reason'
  | 'status_code'
  | 'token_id'

function delayMs(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

function unixNow(): number {
  return Math.floor(Date.now() / 1000)
}

function defaultRange(): { start: number; end: number } {
  const end = unixNow()
  return { start: end - 7 * 24 * 3600, end }
}

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

function statusBadgeClass(code: number | null): string {
  if (code === null) return 'bg-muted text-muted-foreground'
  if (code >= 200 && code < 300) return 'bg-emerald-600/15 text-emerald-700 dark:text-emerald-400'
  if (code >= 400 && code < 500) return 'bg-amber-600/15 text-amber-800 dark:text-amber-300'
  if (code >= 500) return 'bg-red-600/15 text-red-700 dark:text-red-400'
  return 'bg-muted text-muted-foreground'
}

function parseUnixSearchParam(raw: string | null, fallback: number): number {
  if (raw === null || raw === '') return fallback
  const n = Number(raw)
  return Number.isFinite(n) ? n : fallback
}

function urlHasAdvancedFilters(sp: URLSearchParams): boolean {
  return ADVANCED_PARAM_KEYS.some((k) => {
    const v = sp.get(k)
    return v != null && v !== ''
  })
}

export function LogListPage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const defaults = useMemo(() => defaultRange(), [])
  const searchParamsKey = searchParams.toString()

  const limit = PAGE_SIZE
  const offset = Number(searchParams.get('offset') ?? '0')
  const startTime = parseUnixSearchParam(
    searchParams.get('start_time'),
    defaults.start,
  )
  const endTime = parseUnixSearchParam(searchParams.get('end_time'), defaults.end)

  const keyword = searchParams.get('keyword') ?? ''
  const model = searchParams.get('model') ?? ''
  const appId = searchParams.get('app_id') ?? ''
  const finishReason = searchParams.get('finish_reason') ?? ''
  const statusCode = searchParams.get('status_code') ?? ''
  const tokenId = searchParams.get('token_id') ?? ''

  const [draftKeyword, setDraftKeyword] = useState(keyword)
  const [draftModel, setDraftModel] = useState(model)
  const [draftAppId, setDraftAppId] = useState(appId)
  const [draftFinishReason, setDraftFinishReason] = useState(finishReason)
  const [draftStatusCode, setDraftStatusCode] = useState(statusCode)
  const [draftTokenId, setDraftTokenId] = useState(tokenId)

  const [advancedOpen, setAdvancedOpen] = useState(() =>
    typeof window !== 'undefined' &&
    urlHasAdvancedFilters(new URLSearchParams(window.location.search)),
  )

  useEffect(() => {
    const sp = new URLSearchParams(searchParamsKey)
    setDraftKeyword(sp.get('keyword') ?? '')
    setDraftModel(sp.get('model') ?? '')
    setDraftAppId(sp.get('app_id') ?? '')
    setDraftFinishReason(sp.get('finish_reason') ?? '')
    setDraftStatusCode(sp.get('status_code') ?? '')
    setDraftTokenId(sp.get('token_id') ?? '')
  }, [searchParamsKey])

  useEffect(() => {
    if (urlHasAdvancedFilters(new URLSearchParams(searchParamsKey))) {
      setAdvancedOpen(true)
    }
  }, [searchParamsKey])

  const listQuery = useMemo(() => {
    const sc = statusCode.trim()
    const code = sc === '' ? NaN : Number(sc)
    const tid = tokenId.trim() === '' ? NaN : Number(tokenId.trim())
    return {
      start_time: startTime,
      end_time: endTime,
      limit,
      offset,
      ...(keyword.trim() ? { keyword: keyword.trim() } : {}),
      ...(model.trim() ? { model: model.trim() } : {}),
      ...(appId.trim() ? { app_id: appId.trim() } : {}),
      ...(finishReason.trim() ? { finish_reason: finishReason.trim() } : {}),
      ...(Number.isFinite(code) ? { status_code: code } : {}),
      ...(Number.isFinite(tid) ? { token_id: tid } : {}),
    }
  }, [
    startTime,
    endTime,
    limit,
    offset,
    keyword,
    model,
    appId,
    finishReason,
    statusCode,
    tokenId,
  ])

  const { data, isLoading, isError, refetch, isFetching } = useAuditLogList(listQuery)
  const exportMutation = useExportAuditLogs()

  const refreshBusyRef = useRef(false)
  const [manualRefreshSpin, setManualRefreshSpin] = useState(false)

  const handleRefreshList = useCallback(async () => {
    if (refreshBusyRef.current) return
    refreshBusyRef.current = true
    setManualRefreshSpin(true)
    try {
      await Promise.all([refetch(), delayMs(REFRESH_SPIN_MIN_MS)])
    } finally {
      refreshBusyRef.current = false
      setManualRefreshSpin(false)
    }
  }, [refetch])

  const listFetchSpin = isFetching || manualRefreshSpin

  const buildAppliedParams = useCallback(
    (opts: {
      start: number
      end: number
      off: string
      kw: string
      m: string
      app: string
      fr: string
      sc: string
      tid: string
    }) => {
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
    },
    [],
  )

  const applyFilters = useCallback(
    (override?: { statusCode?: string }) => {
      const sc = override?.statusCode ?? draftStatusCode
      setSearchParams(
        buildAppliedParams({
          start: startTime,
          end: endTime,
          off: '0',
          kw: draftKeyword,
          m: draftModel,
          app: draftAppId,
          fr: draftFinishReason,
          sc,
          tid: draftTokenId,
        }),
      )
    },
    [
      startTime,
      endTime,
      draftKeyword,
      draftModel,
      draftAppId,
      draftFinishReason,
      draftStatusCode,
      draftTokenId,
      buildAppliedParams,
      setSearchParams,
    ],
  )

  const resetFilters = useCallback(() => {
    const r = defaultRange()
    setDraftKeyword('')
    setDraftModel('')
    setDraftAppId('')
    setDraftFinishReason('')
    setDraftStatusCode('')
    setDraftTokenId('')
    setSearchParams({
      start_time: String(r.start),
      end_time: String(r.end),
      offset: '0',
    })
  }, [setSearchParams])

  const setPage = (newOffset: number) => {
    const next = new URLSearchParams(searchParams)
    next.set('offset', String(newOffset))
    setSearchParams(next)
  }

  const patchTimeAndOffset = useCallback(
    (patch: { start_time?: number; end_time?: number }) => {
      const next = new URLSearchParams(searchParams)
      if (patch.start_time !== undefined) {
        next.set('start_time', String(patch.start_time))
      }
      if (patch.end_time !== undefined) {
        next.set('end_time', String(patch.end_time))
      }
      next.set('offset', '0')
      setSearchParams(next)
    },
    [searchParams, setSearchParams],
  )

  const removeFilterKey = useCallback(
    (key: AppliedFilterKey) => {
      const next = new URLSearchParams(searchParams)
      next.delete(key)
      next.set('offset', '0')
      setSearchParams(next)
    },
    [searchParams, setSearchParams],
  )

  const handleExport = async () => {
    const created = await exportMutation.mutateAsync({
      start_time: startTime,
      end_time: endTime,
      format: 'csv',
    })
    const blob = await downloadExportFile(created.download_url)
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = `${created.export_id}.csv`
    a.click()
    URL.revokeObjectURL(a.href)
  }

  const total = data?.total ?? 0
  const page = Math.floor(offset / limit) + 1
  const pageCount = Math.max(1, Math.ceil(total / limit))

  const activeChips = useMemo(() => {
    const chips: { key: AppliedFilterKey; label: string }[] = []
    if (keyword.trim()) {
      chips.push({
        key: 'keyword',
        label: `关键词：${keyword.trim().length > 24 ? `${keyword.trim().slice(0, 24)}…` : keyword.trim()}`,
      })
    }
    if (model.trim()) chips.push({ key: 'model', label: `模型：${model.trim()}` })
    if (appId.trim()) chips.push({ key: 'app_id', label: `应用：${appId.trim()}` })
    if (finishReason.trim()) {
      chips.push({ key: 'finish_reason', label: `Finish：${finishReason.trim()}` })
    }
    if (statusCode.trim()) {
      chips.push({ key: 'status_code', label: `HTTP ${statusCode.trim()}` })
    }
    if (tokenId.trim()) chips.push({ key: 'token_id', label: `密钥 ID：${tokenId.trim()}` })
    return chips
  }, [keyword, model, appId, finishReason, statusCode, tokenId])

  const statusPresets = [
    { label: '全部', value: '' },
    { label: '200', value: '200' },
    { label: '429', value: '429' },
    { label: '500', value: '500' },
  ] as const

  return (
    <section className="space-y-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">日志中心</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            请求审计日志 · 开始/结束按本地日历日选择（当日 0:00 至 23:59:59.999，对应
            OpenAPI 的 Unix 秒参数）；修改关键词后请点击查询。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            variant="outline"
            size="icon-sm"
            aria-label="刷新"
            title="刷新"
            aria-busy={listFetchSpin}
            tabIndex={listFetchSpin ? -1 : undefined}
            className={cn(listFetchSpin && 'pointer-events-none')}
            onClick={() => void handleRefreshList()}
          >
            <RefreshCw
              className={cn(listFetchSpin && 'animate-spin')}
              aria-hidden
            />
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={exportMutation.isPending}
            title="导出范围与当前时间选择一致，不含关键词等筛选"
            onClick={() => void handleExport()}
          >
            {exportMutation.isPending ? '导出中…' : '导出 CSV'}
          </Button>
        </div>
      </div>

      <Card className="space-y-4 p-4">
        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault()
            applyFilters()
          }}
        >
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <LogDateField
              label="开始时间"
              mode="start"
              valueUnix={startTime}
              onChangeUnix={(unix) => patchTimeAndOffset({ start_time: unix })}
            />
            <LogDateField
              label="结束时间"
              mode="end"
              valueUnix={endTime}
              onChangeUnix={(unix) => patchTimeAndOffset({ end_time: unix })}
            />
            <label className="text-sm sm:col-span-2">
              <span className="text-muted-foreground">关键词</span>
              <Input
                className="mt-1"
                name="log-keyword"
                value={draftKeyword}
                onChange={(e) => setDraftKeyword(e.target.value)}
                placeholder="request_id / 错误信息 / model（应用查询后生效）"
              />
            </label>
            <label className="text-sm sm:col-span-2 lg:col-span-2">
              <span className="text-muted-foreground">模型</span>
              <Input
                className="mt-1"
                name="log-model"
                value={draftModel}
                onChange={(e) => setDraftModel(e.target.value)}
                placeholder="精确匹配模型名（应用查询后生效）"
              />
            </label>
          </div>

          <div className="border-t border-border pt-3">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="-ml-2 h-8 gap-1 px-2 text-muted-foreground"
              aria-expanded={advancedOpen}
              onClick={() => setAdvancedOpen((o) => !o)}
            >
              <ChevronDown
                className={cn('size-4 transition-transform', advancedOpen && 'rotate-180')}
                aria-hidden
              />
              更多条件（应用、Finish、状态码、密钥 ID）
            </Button>

            {advancedOpen && (
              <div className="mt-3 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
                <label className="text-sm">
                  <span className="text-muted-foreground">应用 (app_id)</span>
                  <Input
                    className="mt-1"
                    value={draftAppId}
                    onChange={(e) => setDraftAppId(e.target.value)}
                    placeholder="请求头 X-App-Id"
                  />
                </label>
                <label className="text-sm">
                  <span className="text-muted-foreground">Finish 原因</span>
                  <Input
                    className="mt-1 font-mono text-sm"
                    value={draftFinishReason}
                    onChange={(e) => setDraftFinishReason(e.target.value)}
                    placeholder="stop 或 stop,length"
                  />
                </label>
                <div className="text-sm sm:col-span-2 lg:col-span-2">
                  <span className="text-muted-foreground">HTTP 状态码</span>
                  <Input
                    className="mt-1 font-mono text-sm"
                    value={draftStatusCode}
                    onChange={(e) => setDraftStatusCode(e.target.value)}
                    placeholder="200"
                    inputMode="numeric"
                  />
                  <div className="mt-2 flex flex-wrap gap-1.5">
                    {statusPresets.map((p) => (
                      <Button
                        key={p.label}
                        type="button"
                        size="sm"
                        variant={
                          (p.value === '' && draftStatusCode.trim() === '') ||
                          draftStatusCode.trim() === p.value
                            ? 'secondary'
                            : 'outline'
                        }
                        className="h-7 text-xs"
                        onClick={() => {
                          if (p.value === '') {
                            setDraftStatusCode('')
                            const next = buildAppliedParams({
                              start: startTime,
                              end: endTime,
                              off: '0',
                              kw: draftKeyword,
                              m: draftModel,
                              app: draftAppId,
                              fr: draftFinishReason,
                              sc: '',
                              tid: draftTokenId,
                            })
                            setSearchParams(next)
                          } else {
                            setDraftStatusCode(p.value)
                            setSearchParams(
                              buildAppliedParams({
                                start: startTime,
                                end: endTime,
                                off: '0',
                                kw: draftKeyword,
                                m: draftModel,
                                app: draftAppId,
                                fr: draftFinishReason,
                                sc: p.value,
                                tid: draftTokenId,
                              }),
                            )
                          }
                        }}
                      >
                        {p.label}
                      </Button>
                    ))}
                  </div>
                </div>
                <label className="text-sm sm:col-span-2 lg:col-span-4">
                  <span className="text-muted-foreground">密钥 ID (token_id)</span>
                  <Input
                    className="mt-1 max-w-md font-mono text-sm"
                    value={draftTokenId}
                    onChange={(e) => setDraftTokenId(e.target.value)}
                    placeholder="与审计日志中的 token_id 一致"
                  />
                </label>
              </div>
            )}
          </div>

          <div className="flex flex-wrap gap-2">
            <Button type="submit">查询</Button>
            <Button type="button" variant="outline" onClick={resetFilters}>
              重置
            </Button>
          </div>
        </form>

        {activeChips.length > 0 && (
          <div className="flex flex-wrap items-center gap-2 border-t border-border pt-3">
            <span className="text-xs text-muted-foreground">已应用</span>
            {activeChips.map((c) => (
              <button
                key={c.key}
                type="button"
                className="inline-flex items-center gap-1 rounded-full border border-border bg-muted/40 px-2.5 py-0.5 text-xs font-medium text-foreground hover:bg-muted/70"
                onClick={() => removeFilterKey(c.key)}
              >
                {c.label}
                <X className="size-3.5 opacity-70" aria-hidden />
                <span className="sr-only">移除{c.label}</span>
              </button>
            ))}
          </div>
        )}
      </Card>

      {isError && (
        <p className="text-sm text-red-600" role="alert">
          加载失败，请检查登录态或稍后重试。
        </p>
      )}

      {isLoading && (
        <p className="text-sm text-muted-foreground">加载中…</p>
      )}

      {!isLoading && data && data.data.length === 0 && (
        <EmptyState
          title="暂无日志"
          description={
            offset > 0 && total > 0
              ? '当前页没有结果，可尝试上一页或调整筛选。'
              : '调整时间范围或筛选条件后再试。'
          }
        />
      )}

      {!isLoading && data && data.data.length > 0 && (
        <div className="overflow-x-auto rounded-lg border border-border">
          <table className="w-full min-w-[1040px] border-collapse text-left text-sm">
            <thead className="border-b border-border bg-muted/40">
              <tr>
                <th scope="col" className="px-3 py-2 font-medium">
                  时间
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  request_id
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  模型
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  应用
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  状态
                </th>
                <th scope="col" className="px-3 py-2 font-medium text-right">
                  prompt
                </th>
                <th scope="col" className="px-3 py-2 font-medium text-right">
                  completion
                </th>
                <th scope="col" className="px-3 py-2 font-medium text-right">
                  合计
                </th>
                <th scope="col" className="px-3 py-2 font-medium text-right">
                  成本 (USD)
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  finish
                </th>
                <th scope="col" className="px-3 py-2 font-medium text-right">
                  耗时 ms
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  操作
                </th>
              </tr>
            </thead>
            <tbody>
              {data.data.map((row) => (
                <tr key={row.request_id} className="border-b border-border/80 hover:bg-muted/30">
                  <td className="whitespace-nowrap px-3 py-2 text-muted-foreground">
                    {formatTime(row.created_at)}
                  </td>
                  <td className="max-w-[140px] truncate px-3 py-2 font-mono text-xs">
                    {row.request_id}
                  </td>
                  <td className="px-3 py-2">{row.model ?? '—'}</td>
                  <td className="px-3 py-2">{row.app_id ?? '—'}</td>
                  <td className="px-3 py-2">
                    {row.status_code !== null ? (
                      <span
                        className={`inline-block rounded px-2 py-0.5 text-xs font-medium ${statusBadgeClass(row.status_code)}`}
                      >
                        {row.status_code}
                      </span>
                    ) : (
                      '—'
                    )}
                  </td>
                  <td className="px-3 py-2 text-right font-mono text-xs">
                    {row.prompt_tokens ?? '—'}
                  </td>
                  <td className="px-3 py-2 text-right font-mono text-xs">
                    {row.completion_tokens ?? '—'}
                  </td>
                  <td className="px-3 py-2 text-right font-mono text-xs">
                    {row.total_tokens ?? '—'}
                  </td>
                  <td className="px-3 py-2 text-right font-mono text-xs">
                    {row.cost != null ? formatCostUsd(row.cost) : '—'}
                  </td>
                  <td className="px-3 py-2 font-mono text-xs">
                    {row.finish_reason ?? '—'}
                  </td>
                  <td className="px-3 py-2 text-right font-mono text-xs">
                    {row.latency_ms ?? '—'}
                  </td>
                  <td className="px-3 py-2">
                    <Link
                      to={`/logs/${encodeURIComponent(row.request_id)}`}
                      className="text-primary underline-offset-4 hover:underline"
                    >
                      详情
                    </Link>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!isLoading && data && data.data.length > 0 && (
        <div className="flex flex-wrap items-center justify-between gap-3 text-sm text-muted-foreground">
          <span>
            共 {total} 条 · 第 {page} / {pageCount} 页
          </span>
          <div className="flex gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={offset <= 0}
              onClick={() => setPage(Math.max(0, offset - limit))}
            >
              上一页
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={offset + limit >= total}
              onClick={() => setPage(offset + limit)}
            >
              下一页
            </Button>
          </div>
        </div>
      )}
    </section>
  )
}
