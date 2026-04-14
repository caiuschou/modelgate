import { Fragment, useCallback, useEffect, useId, useMemo, useRef, useState } from 'react'
import { format } from 'date-fns'
import { zhCN } from 'date-fns/locale'
import { Link, useSearchParams } from 'react-router-dom'
import { ChevronDown, ChevronRight, Info, RefreshCw, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/shared/empty-state'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { AuditLogTimestamp } from '@/features/logs/components/audit-log-timestamp'
import { LogDateField } from '@/features/logs/components/log-date-field'
import { LogListLatencyTooltipCell } from '@/features/logs/components/log-list-latency-tooltip-cell'
import { LogListUsageTooltipCell } from '@/features/logs/components/log-list-usage-tooltip-cell'
import { LogModelPicker } from '@/features/logs/components/log-model-picker'
import {
  downloadExportFile,
  useAuditLogList,
  useAuditThreadList,
  useExportAuditLogs,
} from '@/features/logs/hooks/use-logs'
import {
  auditLogListQuery,
  buildAppliedSearchParams,
  parseUnixSearchParam,
  urlHasAdvancedFilters,
} from '@/features/logs/log-list-filters'
import { rememberLogModel } from '@/features/logs/log-recent-models'
import { formatLatencySeconds } from '@/features/logs/format-latency'
import {
  defaultLogListRange,
  normalizeLogListTimeRange,
  presetLogListRangeToday,
  presetLogListRangeYesterday,
  rollingLogListRange,
} from '@/features/logs/log-list-range'
import { useMyApiKeys } from '@/features/api-keys/hooks/use-api-keys'
import type { ApiKeySummary } from '@/features/api-keys/types'
import { formatCostUsd } from '@/lib/format-cost'
import { cn } from '@/lib/utils'
import type { AuditLogListItem, AuditThreadListItem } from '@/features/logs/types'

const PAGE_SIZE = 20

/** Ensures the refresh icon spins long enough to perceive on fast LAN responses. */
const REFRESH_SPIN_MIN_MS = 280

function formatLogFilterKeyOptionLabel(k: ApiKeySummary): string {
  const status = k.revoked ? '（已撤销）' : k.disabled ? '（已停用）' : ''
  const tail = k.preview ? ` · ${k.preview}` : ` (#${k.id})`
  return `${k.name}${status}${tail}`
}

type AppliedFilterKey =
  | 'keyword'
  | 'model'
  | 'app_id'
  | 'thread_id'
  | 'finish_reason'
  | 'status_code'
  | 'token_id'

function delayMs(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

/** Classic `/logs` URL with this thread id and current range/filters (offset reset). */
function hrefToClassicRequestListForThread(
  threadId: string,
  current: URLSearchParams,
): string {
  const next = new URLSearchParams(current)
  next.set('thread_id', threadId)
  next.set('offset', '0')
  const q = next.toString()
  return q ? `/logs?${q}` : '/logs'
}

function formatListToken(n: number | null | undefined): string {
  if (n == null) return '—'
  return n.toLocaleString()
}

function statusBadgeClass(code: number | null): string {
  if (code === null) return 'bg-muted text-muted-foreground'
  if (code >= 200 && code < 300) return 'bg-emerald-600/15 text-emerald-700 dark:text-emerald-400'
  if (code >= 400 && code < 500) return 'bg-amber-600/15 text-amber-800 dark:text-amber-300'
  if (code >= 500) return 'bg-red-600/15 text-red-700 dark:text-red-400'
  return 'bg-muted text-muted-foreground'
}

export type LogListVariant = 'v1' | 'v2'

export interface LogListPageProps {
  /** @default 'v1' */
  listVariant?: LogListVariant
}

function AuditLogTableRow({
  row,
  density = 'default',
}: {
  row: AuditLogListItem
  /** 嵌套在会话展开区时使用更紧凑的单元格（列与 v1 一致） */
  density?: 'default' | 'compact'
}) {
  const compact = density === 'compact'
  const pad = compact ? 'px-2 py-1.5' : 'px-3 py-2'
  const mono = compact ? 'font-mono text-[11px]' : 'font-mono text-xs'
  const badgeCls = (code: number) =>
    compact
      ? `inline-block rounded px-1.5 py-0.5 text-[11px] font-medium ${statusBadgeClass(code)}`
      : `inline-block rounded px-2 py-0.5 text-xs font-medium ${statusBadgeClass(code)}`

  const timeCell = (
    <td className={cn('whitespace-nowrap text-muted-foreground', pad)}>
      <AuditLogTimestamp unixSeconds={row.created_at} />
    </td>
  )
  const requestIdCell = (
    <td className={cn('max-w-[140px] truncate', pad, mono)}>
      {row.request_id}
    </td>
  )
  const threadCell = (
    <td className={cn('max-w-[160px] truncate', pad, mono)}>
      {row.thread_id ?? '—'}
    </td>
  )
  const modelCell = (
    <td className={cn(pad, compact && 'text-xs')}>{row.model ?? '—'}</td>
  )
  const appCell = (
    <td className={cn(pad, compact && 'text-xs')}>{row.app_id ?? '—'}</td>
  )
  const statusCell = (
    <td className={pad}>
      {row.status_code !== null ? (
        <span className={badgeCls(row.status_code)}>{row.status_code}</span>
      ) : (
        '—'
      )}
    </td>
  )
  const usageCell = (
    <td className={cn('text-right', mono, pad)}>
      <LogListUsageTooltipCell row={row}>
        <div className="flex flex-col items-end gap-0.5 leading-tight">
          <div>
            <span className="text-muted-foreground">P</span>{' '}
            {formatListToken(row.prompt_tokens)}
          </div>
          <div>
            <span className="text-muted-foreground">C</span>{' '}
            {formatListToken(row.completion_tokens)}
          </div>
          <div>
            <span className="text-muted-foreground">Σ</span>{' '}
            {formatListToken(row.total_tokens)}
          </div>
        </div>
      </LogListUsageTooltipCell>
    </td>
  )
  const costCell = (
    <td className={cn('text-right', mono, pad)}>
      {row.cost != null ? formatCostUsd(row.cost) : '—'}
    </td>
  )
  const finishCell = (
    <td className={cn(mono, pad)}>{row.finish_reason ?? '—'}</td>
  )
  const latencyCell = (
    <td className={cn('text-right', mono, pad)}>
      <LogListLatencyTooltipCell row={row}>
        {formatLatencySeconds(row.latency_ms)}
      </LogListLatencyTooltipCell>
    </td>
  )
  const actionCell = (
    <td className={pad}>
      <Link
        to={`/logs/${encodeURIComponent(row.request_id)}`}
        className="text-primary underline-offset-4 hover:underline"
      >
        详情
      </Link>
    </td>
  )

  return (
    <tr
      className={cn(
        'border-b hover:bg-muted/30',
        compact ? 'border-border/60 last:border-0' : 'border-border/80',
      )}
    >
      {timeCell}
      {requestIdCell}
      {modelCell}
      {appCell}
      {threadCell}
      {statusCell}
      {usageCell}
      {costCell}
      {finishCell}
      {latencyCell}
      {actionCell}
    </tr>
  )
}

const THREAD_CHILD_LIMIT = 50

type ThreadRowFilterContext = {
  startTime: number
  endTime: number
  keyword: string
  model: string
  appId: string
  finishReason: string
  statusCode: string
  tokenId: string
}

function AuditThreadTableGroup({
  row,
  requestListHref,
  expanded,
  onToggleExpand,
  filterCtx,
}: {
  row: AuditThreadListItem
  requestListHref: string
  expanded: boolean
  onToggleExpand: () => void
  filterCtx: ThreadRowFilterContext
}) {
  const threadRequestsQuery = useMemo(
    () =>
      auditLogListQuery({
        startTime: filterCtx.startTime,
        endTime: filterCtx.endTime,
        limit: THREAD_CHILD_LIMIT,
        offset: 0,
        keyword: filterCtx.keyword,
        model: filterCtx.model,
        appId: filterCtx.appId,
        threadId: row.thread_id,
        finishReason: filterCtx.finishReason,
        statusCode: filterCtx.statusCode,
        tokenId: filterCtx.tokenId,
      }),
    [
      filterCtx.appId,
      filterCtx.endTime,
      filterCtx.finishReason,
      filterCtx.keyword,
      filterCtx.model,
      filterCtx.startTime,
      filterCtx.statusCode,
      filterCtx.tokenId,
      row.thread_id,
    ],
  )

  const childList = useAuditLogList(threadRequestsQuery, {
    enabled: expanded,
  })

  return (
    <Fragment>
      <tr className="border-b border-border/80 hover:bg-muted/30">
        <td className="w-10 px-2 py-2 align-middle">
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className="size-8 shrink-0"
            aria-expanded={expanded}
            aria-label={expanded ? '收起该会话下的请求' : '展开该会话下的请求'}
            onClick={onToggleExpand}
          >
            {expanded ? (
              <ChevronDown className="size-4" aria-hidden />
            ) : (
              <ChevronRight className="size-4" aria-hidden />
            )}
          </Button>
        </td>
        <td className="whitespace-nowrap px-3 py-2 text-muted-foreground">
          <AuditLogTimestamp unixSeconds={row.last_seen_at} />
        </td>
        <td
          className="max-w-[min(100vw,24rem)] truncate px-3 py-2 font-mono text-xs"
          title={row.thread_id}
        >
          {row.thread_id}
        </td>
        <td className="px-3 py-2 text-right tabular-nums">{row.request_count}</td>
        <td className="whitespace-nowrap px-3 py-2 text-muted-foreground">
          <AuditLogTimestamp unixSeconds={row.first_seen_at} />
        </td>
        <td className="px-3 py-2">
          <Link
            to={requestListHref}
            className="text-primary underline-offset-4 hover:underline"
          >
            请求列表
          </Link>
        </td>
      </tr>
      {expanded ? (
        <tr className="border-b border-border/80 bg-muted/20">
          <td colSpan={6} className="px-3 py-2 align-top">
            {childList.isLoading ? (
              <p className="text-xs text-muted-foreground">加载中…</p>
            ) : childList.isError ? (
              <p className="text-xs text-red-600" role="alert">
                请求列表加载失败，请稍后重试。
              </p>
            ) : !childList.data?.data.length ? (
              <p className="text-xs text-muted-foreground">当前筛选下暂无请求。</p>
            ) : (
              <div className="rounded-md border border-border/80 bg-background/80">
                <TooltipProvider delayDuration={250} skipDelayDuration={200}>
                  <div className="overflow-x-auto">
                    <table className="w-full min-w-[1000px] border-collapse text-left text-xs">
                      <thead className="border-b border-border bg-muted/50">
                        <tr>
                          <th scope="col" className="px-2 py-1.5 font-medium">
                            时间
                          </th>
                          <th scope="col" className="px-2 py-1.5 font-medium">
                            request_id
                          </th>
                          <th scope="col" className="px-2 py-1.5 font-medium">
                            模型
                          </th>
                          <th scope="col" className="px-2 py-1.5 font-medium">
                            应用
                          </th>
                          <th scope="col" className="px-2 py-1.5 font-medium">
                            会话
                          </th>
                          <th scope="col" className="px-2 py-1.5 font-medium">
                            状态
                          </th>
                          <th
                            scope="col"
                            className="px-2 py-1.5 text-right font-medium"
                          >
                            用量 (P/C/Σ)
                          </th>
                          <th
                            scope="col"
                            className="px-2 py-1.5 text-right font-medium"
                          >
                            成本 (USD)
                          </th>
                          <th scope="col" className="px-2 py-1.5 font-medium">
                            finish
                          </th>
                          <th
                            scope="col"
                            className="px-2 py-1.5 text-right font-medium"
                          >
                            耗时 (s)
                          </th>
                          <th scope="col" className="px-2 py-1.5 font-medium">
                            操作
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {childList.data.data.map((r) => (
                          <AuditLogTableRow
                            key={r.request_id}
                            row={r}
                            density="compact"
                          />
                        ))}
                      </tbody>
                    </table>
                  </div>
                </TooltipProvider>
                {childList.data.total > THREAD_CHILD_LIMIT ? (
                  <p className="border-t border-border/80 px-2 py-1.5 text-[11px] text-muted-foreground">
                    仅显示前 {THREAD_CHILD_LIMIT} 条，共 {childList.data.total}{' '}
                    条；完整列表请使用「请求列表」。
                  </p>
                ) : null}
              </div>
            )}
          </td>
        </tr>
      ) : null}
    </Fragment>
  )
}

export function LogListPage({ listVariant = 'v1' }: LogListPageProps) {
  const modelFieldId = useId()
  const tokenFieldId = useId()
  const [searchParams, setSearchParams] = useSearchParams()
  const { data: apiKeysRes, isPending: apiKeysLoading } = useMyApiKeys()
  const defaults = useMemo(() => defaultLogListRange(), [])
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
  const threadId = searchParams.get('thread_id') ?? ''
  const finishReason = searchParams.get('finish_reason') ?? ''
  const statusCode = searchParams.get('status_code') ?? ''
  const tokenId = searchParams.get('token_id') ?? ''

  const [draftKeyword, setDraftKeyword] = useState(keyword)
  const [draftModel, setDraftModel] = useState(model)
  const [draftAppId, setDraftAppId] = useState(appId)
  const [draftThreadId, setDraftThreadId] = useState(threadId)
  const [draftFinishReason, setDraftFinishReason] = useState(finishReason)
  const [draftStatusCode, setDraftStatusCode] = useState(statusCode)
  const [draftTokenId, setDraftTokenId] = useState(tokenId)

  const [advancedOpen, setAdvancedOpen] = useState(() =>
    typeof window !== 'undefined' &&
    urlHasAdvancedFilters(new URLSearchParams(window.location.search)),
  )

  const [recentModelsStorageRev, setRecentModelsStorageRev] = useState(0)
  /** v2: at most one expanded session row for inline request list */
  const [expandedThreadId, setExpandedThreadId] = useState<string | null>(null)

  useEffect(() => {
    const sp = new URLSearchParams(searchParamsKey)
    setDraftKeyword(sp.get('keyword') ?? '')
    setDraftModel(sp.get('model') ?? '')
    setDraftAppId(sp.get('app_id') ?? '')
    setDraftThreadId(sp.get('thread_id') ?? '')
    setDraftFinishReason(sp.get('finish_reason') ?? '')
    setDraftStatusCode(sp.get('status_code') ?? '')
    setDraftTokenId(sp.get('token_id') ?? '')
  }, [searchParamsKey])

  useEffect(() => {
    if (urlHasAdvancedFilters(new URLSearchParams(searchParamsKey))) {
      setAdvancedOpen(true)
    }
  }, [searchParamsKey])

  const listQuery = useMemo(
    () =>
      auditLogListQuery({
        startTime,
        endTime,
        limit,
        offset,
        keyword,
        model,
        appId,
        threadId,
        finishReason,
        statusCode,
        tokenId,
      }),
    [
      startTime,
      endTime,
      limit,
      offset,
      keyword,
      model,
      appId,
      threadId,
      finishReason,
      statusCode,
      tokenId,
    ],
  )

  const threadExpandFilterCtx = useMemo(
    (): ThreadRowFilterContext => ({
      startTime,
      endTime,
      keyword,
      model,
      appId,
      finishReason,
      statusCode,
      tokenId,
    }),
    [
      startTime,
      endTime,
      keyword,
      model,
      appId,
      finishReason,
      statusCode,
      tokenId,
    ],
  )

  const listV1 = useAuditLogList(listQuery, { enabled: listVariant === 'v1' })
  const listV2 = useAuditThreadList(listQuery, { enabled: listVariant === 'v2' })
  useEffect(() => {
    if (listVariant !== 'v2' || !listV2.data?.data.length) return
    const ids = new Set(listV2.data.data.map((r) => r.thread_id))
    if (expandedThreadId && !ids.has(expandedThreadId)) {
      setExpandedThreadId(null)
    }
  }, [listVariant, listV2.data, expandedThreadId])

  const data = listVariant === 'v1' ? listV1.data : listV2.data
  const isLoading = listVariant === 'v1' ? listV1.isLoading : listV2.isLoading
  const isError = listVariant === 'v1' ? listV1.isError : listV2.isError
  const isFetching = listVariant === 'v1' ? listV1.isFetching : listV2.isFetching
  const exportMutation = useExportAuditLogs()

  const refreshBusyRef = useRef(false)
  const [manualRefreshSpin, setManualRefreshSpin] = useState(false)

  const handleRefreshList = useCallback(async () => {
    if (refreshBusyRef.current) return
    refreshBusyRef.current = true
    setManualRefreshSpin(true)
    try {
      await Promise.all([
        listV1.refetch(),
        listV2.refetch(),
        delayMs(REFRESH_SPIN_MIN_MS),
      ])
    } finally {
      refreshBusyRef.current = false
      setManualRefreshSpin(false)
    }
  }, [listV1.refetch, listV2.refetch])

  const listFetchSpin = isFetching || manualRefreshSpin

  const applyFilters = useCallback(
    (override?: { statusCode?: string }) => {
      const sc = override?.statusCode ?? draftStatusCode
      const m = draftModel.trim()
      if (m) {
        rememberLogModel(m)
        setRecentModelsStorageRev((n) => n + 1)
      }
      setSearchParams(
        buildAppliedSearchParams({
          start: startTime,
          end: endTime,
          off: '0',
          kw: draftKeyword,
          m: draftModel,
          app: draftAppId,
          thread: draftThreadId,
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
      draftThreadId,
      draftFinishReason,
      draftStatusCode,
      draftTokenId,
      setSearchParams,
    ],
  )

  const resetFilters = useCallback(() => {
    const r = defaultLogListRange()
    setDraftKeyword('')
    setDraftModel('')
    setDraftAppId('')
    setDraftThreadId('')
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
    (patch: { start_time?: number; end_time?: number; which: 'start' | 'end' }) => {
      const nextStart = patch.start_time ?? startTime
      const nextEnd = patch.end_time ?? endTime
      const norm = normalizeLogListTimeRange(nextStart, nextEnd, patch.which)
      const next = new URLSearchParams(searchParams)
      next.set('start_time', String(norm.start))
      next.set('end_time', String(norm.end))
      next.set('offset', '0')
      setSearchParams(next)
    },
    [searchParams, setSearchParams, startTime, endTime],
  )

  const applyTimePreset = useCallback(
    (preset: 'today' | 'yesterday' | 'last7' | 'last30') => {
      const at = new Date()
      const range =
        preset === 'today'
          ? presetLogListRangeToday(at)
          : preset === 'yesterday'
            ? presetLogListRangeYesterday(at)
            : preset === 'last7'
              ? defaultLogListRange(at)
              : rollingLogListRange(30, at)
      const next = new URLSearchParams(searchParams)
      next.set('start_time', String(range.start))
      next.set('end_time', String(range.end))
      next.set('offset', '0')
      setSearchParams(next)
    },
    [searchParams, setSearchParams],
  )

  const activeTimePreset = useMemo((): 'today' | 'yesterday' | 'last7' | 'last30' | null => {
    const cur = { start: startTime, end: endTime }
    const at = new Date()
    const today = presetLogListRangeToday(at)
    if (cur.start === today.start && cur.end === today.end) return 'today'
    const yesterday = presetLogListRangeYesterday(at)
    if (cur.start === yesterday.start && cur.end === yesterday.end) return 'yesterday'
    const last7 = defaultLogListRange(at)
    if (cur.start === last7.start && cur.end === last7.end) return 'last7'
    const last30 = rollingLogListRange(30, at)
    if (cur.start === last30.start && cur.end === last30.end) return 'last30'
    return null
  }, [startTime, endTime])

  const logRangeSummaryLine = useMemo(() => {
    const s = new Date(startTime * 1000)
    const e = new Date(endTime * 1000)
    const rawDays = (endTime - startTime) / (24 * 3600)
    const spanLabel =
      Math.abs(rawDays - Math.round(rawDays)) < 1e-3
        ? `${Math.round(rawDays)} 天`
        : `约 ${rawDays.toFixed(1)} 天`
    return `${format(s, 'M月d日 HH:mm', { locale: zhCN })} — ${format(e, 'M月d日 HH:mm', { locale: zhCN })} · 跨度 ${spanLabel}（本地时区）`
  }, [startTime, endTime])

  const logRangeSummaryCompact = useMemo(() => {
    const s = new Date(startTime * 1000)
    const e = new Date(endTime * 1000)
    const rawDays = (endTime - startTime) / (24 * 3600)
    const spanLabel =
      Math.abs(rawDays - Math.round(rawDays)) < 1e-3
        ? `${Math.round(rawDays)} 天`
        : `约 ${rawDays.toFixed(1)} 天`
    return `${format(s, 'yyyy/M/d', { locale: zhCN })} — ${format(e, 'yyyy/M/d', { locale: zhCN })} · ${spanLabel}`
  }, [startTime, endTime])

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

  const apiKeyRows = useMemo(() => apiKeysRes?.data ?? [], [apiKeysRes?.data])

  const tokenIdNumeric = useMemo(() => {
    const t = tokenId.trim()
    const n = Number(t)
    return Number.isFinite(n) ? n : null
  }, [tokenId])

  const tokenChipLabel = useMemo(() => {
    if (!tokenId.trim()) return ''
    if (tokenIdNumeric != null) {
      const row = apiKeyRows.find((k) => k.id === tokenIdNumeric)
      if (row) return `密钥：${row.name}`
    }
    return `密钥 ID：${tokenId.trim()}`
  }, [tokenId, tokenIdNumeric, apiKeyRows])

  const showOrphanTokenOption = useMemo(() => {
    const d = draftTokenId.trim()
    if (!d) return false
    const n = Number(d)
    if (Number.isFinite(n) && apiKeyRows.some((k) => k.id === n)) return false
    return true
  }, [draftTokenId, apiKeyRows])

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
    if (threadId.trim()) {
      chips.push({ key: 'thread_id', label: `会话：${threadId.trim()}` })
    }
    if (finishReason.trim()) {
      chips.push({ key: 'finish_reason', label: `Finish：${finishReason.trim()}` })
    }
    if (statusCode.trim()) {
      chips.push({ key: 'status_code', label: `HTTP ${statusCode.trim()}` })
    }
    if (tokenId.trim()) chips.push({ key: 'token_id', label: tokenChipLabel })
    return chips
  }, [
    keyword,
    model,
    appId,
    threadId,
    finishReason,
    statusCode,
    tokenId,
    tokenChipLabel,
  ])

  const statusPresets = [
    { label: '全部', value: '' },
    { label: '200', value: '200' },
    { label: '429', value: '429' },
    { label: '500', value: '500' },
  ] as const

  return (
    <TooltipProvider delayDuration={280}>
    <section className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">
            {listVariant === 'v2' ? '日志中心（新版）' : '日志中心'}
          </h1>
          <p className="mt-1 text-xs text-muted-foreground sm:text-sm">
            {listVariant === 'v2'
              ? '每行一个会话 (thread)：筛选与经典列表一致；点击行首箭头可展开查看该会话下请求（最多 50 条）。请求数为维表累计次数。'
              : '审计请求日志 · 默认最近 7×24 小时至今日日末。'}
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
          <Button type="button" variant="outline" size="sm" asChild>
            <Link
              to={
                listVariant === 'v2'
                  ? `/logs${searchParams.toString() ? `?${searchParams.toString()}` : ''}`
                  : `/logs/v2${searchParams.toString() ? `?${searchParams.toString()}` : ''}`
              }
              replace={false}
            >
              {listVariant === 'v2' ? '经典列表' : '新版列表'}
            </Link>
          </Button>
        </div>
      </div>

      <Card className="overflow-hidden p-0">
        <form
          className="divide-y divide-border"
          onSubmit={(e) => {
            e.preventDefault()
            applyFilters()
          }}
        >
          <div className="space-y-2 p-3 sm:p-4">
            <div className="flex flex-col gap-2 lg:flex-row lg:items-end lg:gap-3">
              <div className="grid min-w-0 flex-1 gap-2 sm:max-w-xl sm:grid-cols-2">
                <LogDateField
                  compact
                  label="开始"
                  mode="start"
                  valueUnix={startTime}
                  onChangeUnix={(unix) => patchTimeAndOffset({ start_time: unix, which: 'start' })}
                />
                <LogDateField
                  compact
                  label="结束"
                  mode="end"
                  valueUnix={endTime}
                  onChangeUnix={(unix) => patchTimeAndOffset({ end_time: unix, which: 'end' })}
                />
              </div>
              <label className="flex w-full flex-col gap-0.5 lg:w-36 lg:shrink-0">
                <span className="text-xs text-muted-foreground">快捷范围</span>
                <select
                  aria-label="快捷时间范围"
                  className={cn(
                    'h-8 w-full rounded-lg border border-input bg-transparent px-2 text-xs outline-none transition-colors',
                    'focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40',
                    'dark:bg-input/30',
                  )}
                  value={activeTimePreset ?? 'custom'}
                  onChange={(e) => {
                    const v = e.target.value
                    if (v === 'custom') return
                    applyTimePreset(v as 'today' | 'yesterday' | 'last7' | 'last30')
                  }}
                >
                  <option value="custom">自定义区间</option>
                  <option value="today">今天</option>
                  <option value="yesterday">昨天</option>
                  <option value="last7">最近 7 天</option>
                  <option value="last30">最近 30 天</option>
                </select>
              </label>
            </div>
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className="min-w-0 flex-1 truncate tabular-nums" title={logRangeSummaryLine}>
                {logRangeSummaryCompact}
              </span>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    className="inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
                    aria-label="完整时间区间说明"
                  >
                    <Info className="size-3.5" aria-hidden />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="bottom" align="end" className="max-w-xs text-left">
                  {logRangeSummaryLine}
                </TooltipContent>
              </Tooltip>
            </div>

            <div className="grid gap-2 border-t border-dashed border-border/80 pt-2 lg:grid-cols-2">
              <label className="text-xs sm:text-sm">
                <span className="text-muted-foreground">关键词</span>
                <Input
                  className="mt-0.5 h-8 text-sm"
                  name="log-keyword"
                  value={draftKeyword}
                  onChange={(e) => setDraftKeyword(e.target.value)}
                  placeholder="request_id / 错误 / model"
                />
              </label>
              <div className="text-xs sm:text-sm">
                <label htmlFor={modelFieldId} className="text-muted-foreground">
                  模型
                </label>
                <LogModelPicker
                  id={modelFieldId}
                  value={draftModel}
                  onChange={setDraftModel}
                  storageRevision={recentModelsStorageRev}
                />
              </div>
            </div>
          </div>

          <div className="p-3 sm:p-4">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 gap-1 px-2 text-muted-foreground hover:text-foreground"
              aria-expanded={advancedOpen}
              onClick={() => setAdvancedOpen((o) => !o)}
            >
              <ChevronDown
                className={cn('size-4 transition-transform', advancedOpen && 'rotate-180')}
                aria-hidden
              />
              更多条件（应用、会话、Finish、状态码、密钥）
            </Button>

            {advancedOpen && (
              <div className="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
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
                  <span className="text-muted-foreground">会话 (thread_id)</span>
                  <Input
                    className="mt-1"
                    value={draftThreadId}
                    onChange={(e) => setDraftThreadId(e.target.value)}
                    placeholder="请求头 X-Thread-Id"
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
                            const next = buildAppliedSearchParams({
                              start: startTime,
                              end: endTime,
                              off: '0',
                              kw: draftKeyword,
                              m: draftModel,
                              app: draftAppId,
                              thread: draftThreadId,
                              fr: draftFinishReason,
                              sc: '',
                              tid: draftTokenId,
                            })
                            setSearchParams(next)
                          } else {
                            setDraftStatusCode(p.value)
                            setSearchParams(
                              buildAppliedSearchParams({
                                start: startTime,
                                end: endTime,
                                off: '0',
                                kw: draftKeyword,
                                m: draftModel,
                                app: draftAppId,
                                thread: draftThreadId,
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
                <label htmlFor={tokenFieldId} className="text-sm sm:col-span-2 lg:col-span-4">
                  <span className="text-muted-foreground">密钥</span>
                  <select
                    id={tokenFieldId}
                    name="log-token-id"
                    className={cn(
                      'mt-1 w-full max-w-full font-mono text-sm',
                      'h-8 rounded-lg border border-input bg-transparent px-2.5 py-1 outline-none transition-colors',
                      'focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50',
                      'disabled:cursor-not-allowed disabled:opacity-50',
                      'md:text-sm dark:bg-input/30',
                    )}
                    value={draftTokenId}
                    onChange={(e) => setDraftTokenId(e.target.value)}
                  >
                    <option value="">全部</option>
                    {showOrphanTokenOption ? (
                      <option value={draftTokenId.trim()}>
                        {Number.isFinite(Number(draftTokenId.trim()))
                          ? `ID ${draftTokenId.trim()}（不在当前列表）`
                          : `「${draftTokenId.trim()}」（不在当前列表）`}
                      </option>
                    ) : null}
                    {apiKeyRows.map((k) => (
                      <option key={k.id} value={String(k.id)}>
                        {formatLogFilterKeyOptionLabel(k)}
                      </option>
                    ))}
                  </select>
                  {apiKeysLoading ? (
                    <p className="mt-1 text-xs text-muted-foreground">正在加载密钥列表…</p>
                  ) : apiKeyRows.length === 0 ? (
                    <p className="mt-1 text-xs text-muted-foreground">
                      暂无密钥，请先在「API 密钥」中创建。
                    </p>
                  ) : null}
                </label>
              </div>
            )}
          </div>

          <div className="flex flex-col gap-2 bg-muted/20 p-3 sm:flex-row sm:items-center sm:justify-between sm:gap-3 sm:px-4">
            <p className="hidden text-[11px] leading-snug text-muted-foreground sm:block sm:max-w-[min(100%,20rem)]">
              改时间即查列表；关键词与更多条件需点查询。
            </p>
            <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
              <Button type="submit" size="sm" title="将关键词、模型与更多条件写入地址栏并查询">
                查询
              </Button>
              <Button type="button" variant="outline" size="sm" onClick={resetFilters}>
                重置
              </Button>
            </div>
          </div>
        </form>

        {activeChips.length > 0 && (
          <div className="flex flex-wrap items-center gap-2 border-t border-border bg-muted/15 px-3 py-2 sm:px-4">
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

      {!isLoading && data && data.data.length > 0 && listVariant === 'v1' && (
        <TooltipProvider delayDuration={250} skipDelayDuration={200}>
          <div className="overflow-x-auto rounded-lg border border-border">
          <table className="w-full min-w-[1000px] border-collapse text-left text-sm">
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
                  会话
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  状态
                </th>
                <th scope="col" className="px-3 py-2 font-medium text-right">
                  用量 (P/C/Σ)
                </th>
                <th scope="col" className="px-3 py-2 font-medium text-right">
                  成本 (USD)
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  finish
                </th>
                <th scope="col" className="px-3 py-2 font-medium text-right">
                  耗时 (s)
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  操作
                </th>
              </tr>
            </thead>
            <tbody>
              {data.data.map((row) => (
                <AuditLogTableRow key={row.request_id} row={row} />
              ))}
            </tbody>
          </table>
          </div>
        </TooltipProvider>
      )}

      {!isLoading && listV2.data && listV2.data.data.length > 0 && listVariant === 'v2' && (
        <div className="overflow-x-auto rounded-lg border border-border">
          <table className="w-full min-w-[720px] border-collapse text-left text-sm">
            <thead className="border-b border-border bg-muted/40">
              <tr>
                <th
                  scope="col"
                  className="w-10 px-2 py-2"
                  aria-label="展开"
                />
                <th scope="col" className="px-3 py-2 font-medium">
                  最后活动
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  会话 (thread)
                </th>
                <th scope="col" className="px-3 py-2 text-right font-medium">
                  请求数
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  首次活动
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  操作
                </th>
              </tr>
            </thead>
            <tbody>
              {listV2.data.data.map((row) => (
                <AuditThreadTableGroup
                  key={`${row.user_id}-${row.team_id ?? 'p'}-${row.thread_id}`}
                  row={row}
                  requestListHref={hrefToClassicRequestListForThread(
                    row.thread_id,
                    searchParams,
                  )}
                  expanded={expandedThreadId === row.thread_id}
                  onToggleExpand={() =>
                    setExpandedThreadId((cur) =>
                      cur === row.thread_id ? null : row.thread_id,
                    )
                  }
                  filterCtx={threadExpandFilterCtx}
                />
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
    </TooltipProvider>
  )
}
