import { useCallback, useMemo, useState } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/shared/empty-state'
import {
  downloadExportFile,
  useAuditLogList,
  useExportAuditLogs,
} from '@/features/logs/hooks/use-logs'

const PAGE_SIZE = 20

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

export function LogListPage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const defaults = useMemo(() => defaultRange(), [])

  const limit = PAGE_SIZE
  const offset = Number(searchParams.get('offset') ?? '0')
  const startTime = Number(searchParams.get('start_time') ?? String(defaults.start))
  const endTime = Number(searchParams.get('end_time') ?? String(defaults.end))

  const [keyword, setKeyword] = useState(searchParams.get('keyword') ?? '')
  const [model, setModel] = useState(searchParams.get('model') ?? '')
  const [appId, setAppId] = useState(searchParams.get('app_id') ?? '')
  const [finishReason, setFinishReason] = useState(
    searchParams.get('finish_reason') ?? '',
  )
  const [statusCode, setStatusCode] = useState(searchParams.get('status_code') ?? '')

  const listQuery = useMemo(() => {
    const sc = statusCode.trim()
    const code = sc === '' ? NaN : Number(sc)
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
    }
  }, [startTime, endTime, limit, offset, keyword, model, appId, finishReason, statusCode])

  const { data, isLoading, isError, refetch } = useAuditLogList(listQuery)
  const exportMutation = useExportAuditLogs()

  const applyFilters = useCallback(() => {
    const next = new URLSearchParams()
    next.set('start_time', String(startTime))
    next.set('end_time', String(endTime))
    next.set('offset', '0')
    if (keyword.trim()) next.set('keyword', keyword.trim())
    if (model.trim()) next.set('model', model.trim())
    if (appId.trim()) next.set('app_id', appId.trim())
    if (finishReason.trim()) next.set('finish_reason', finishReason.trim())
    if (statusCode.trim()) next.set('status_code', statusCode.trim())
    setSearchParams(next)
  }, [
    startTime,
    endTime,
    keyword,
    model,
    appId,
    finishReason,
    statusCode,
    setSearchParams,
  ])

  const resetFilters = useCallback(() => {
    const r = defaultRange()
    setKeyword('')
    setModel('')
    setAppId('')
    setFinishReason('')
    setStatusCode('')
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

  return (
    <section className="space-y-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">日志中心</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            请求审计日志 · 时间范围为 Unix 秒级，与 OpenAPI 一致
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="outline" size="sm" onClick={() => refetch()}>
            刷新
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={exportMutation.isPending}
            onClick={() => void handleExport()}
          >
            {exportMutation.isPending ? '导出中…' : '导出 CSV'}
          </Button>
        </div>
      </div>

      <Card className="space-y-4 p-4">
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <label className="text-sm">
            <span className="text-muted-foreground">开始时间 (Unix 秒)</span>
            <Input
              className="mt-1 font-mono text-sm"
              value={String(startTime)}
              onChange={(e) => {
                const next = new URLSearchParams(searchParams)
                next.set('start_time', e.target.value)
                setSearchParams(next)
              }}
            />
          </label>
          <label className="text-sm">
            <span className="text-muted-foreground">结束时间 (Unix 秒)</span>
            <Input
              className="mt-1 font-mono text-sm"
              value={String(endTime)}
              onChange={(e) => {
                const next = new URLSearchParams(searchParams)
                next.set('end_time', e.target.value)
                setSearchParams(next)
              }}
            />
          </label>
          <label className="text-sm sm:col-span-2">
            <span className="text-muted-foreground">关键词</span>
            <Input
              className="mt-1"
              value={keyword}
              onChange={(e) => setKeyword(e.target.value)}
              placeholder="request_id / 错误信息 / model"
            />
          </label>
          <label className="text-sm">
            <span className="text-muted-foreground">模型</span>
            <Input
              className="mt-1"
              value={model}
              onChange={(e) => setModel(e.target.value)}
            />
          </label>
          <label className="text-sm">
            <span className="text-muted-foreground">应用 (app_id)</span>
            <Input
              className="mt-1"
              value={appId}
              onChange={(e) => setAppId(e.target.value)}
              placeholder="请求头 X-App-Id"
            />
          </label>
          <label className="text-sm">
            <span className="text-muted-foreground">Finish 原因</span>
            <Input
              className="mt-1 font-mono text-sm"
              value={finishReason}
              onChange={(e) => setFinishReason(e.target.value)}
              placeholder="stop 或 stop,length"
            />
          </label>
          <label className="text-sm">
            <span className="text-muted-foreground">HTTP 状态码</span>
            <Input
              className="mt-1 font-mono text-sm"
              value={statusCode}
              onChange={(e) => setStatusCode(e.target.value)}
              placeholder="200"
            />
          </label>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button type="button" onClick={applyFilters}>
            查询
          </Button>
          <Button type="button" variant="outline" onClick={resetFilters}>
            重置
          </Button>
        </div>
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
        <EmptyState title="暂无日志" description="调整时间范围或筛选条件后再试。" />
      )}

      {!isLoading && data && data.data.length > 0 && (
        <div className="overflow-x-auto rounded-lg border border-border">
          <table className="w-full min-w-[960px] border-collapse text-left text-sm">
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
