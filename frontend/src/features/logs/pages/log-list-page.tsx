import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react'
import { format } from 'date-fns'
import { zhCN } from 'date-fns/locale'
import { Link, useSearchParams } from 'react-router-dom'
import {
  ChevronDown,
  FileText,
  Info,
  List,
  RefreshCw,
  X,
} from 'lucide-react'
import { Button, buttonVariants } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
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
import {
  formatCumulativeLatencyMs,
  formatLatencySeconds,
} from '@/features/logs/format-latency'
import { formatThreadActivitySpan } from '@/features/logs/format-thread-activity-span'
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
import { LogDetailContent } from '@/features/logs/pages/log-detail-page'

const PAGE_SIZE = 20

/** 日志列表右侧抽屉宽度（与 e2e 中会话/详情 Sheet 断言一致） */
const logSheetPanelStyle = {
  width: 'calc(100vw * 2 / 3)',
  maxWidth: 'calc(100vw * 2 / 3)',
} as const

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

function formatSessionTokenField(v: number | undefined | null): string {
  if (v == null || Number.isNaN(v)) return '—'
  return v.toLocaleString()
}

/** v2 会话行：合并「首次 / 最后一次」请求时间；时间与标签字号统一。 */
const activityTimeClassName =
  'text-sm cursor-default underline decoration-dotted decoration-muted-foreground/50 underline-offset-2'

function AuditThreadActivityCell({ row }: { row: AuditThreadListItem }) {
  const same = row.first_seen_at === row.last_seen_at
  const spanSec = Math.max(0, row.last_seen_at - row.first_seen_at)
  const spanLabel = formatThreadActivitySpan(spanSec)
  const spanLine = (
    <span
      className="text-[11px] tabular-nums text-muted-foreground/85"
      title="本会话末次与首次请求时间的墙钟间隔（与会话内各请求累计耗时不同）"
    >
      跨度 · {spanLabel}
    </span>
  )
  return (
    <td className="px-3 py-2 align-middle text-sm text-muted-foreground">
      {same ? (
        <div className="flex flex-col gap-1">
          <AuditLogTimestamp
            unixSeconds={row.last_seen_at}
            className={activityTimeClassName}
          />
          {spanLine}
        </div>
      ) : (
        <div className="flex flex-col gap-1">
          <div className="flex flex-col gap-0.5">
            <div className="flex min-w-0 items-baseline gap-1.5 whitespace-nowrap">
              <span className="shrink-0 text-xs font-medium text-muted-foreground/90">
                末
              </span>
              <AuditLogTimestamp
                unixSeconds={row.last_seen_at}
                className={activityTimeClassName}
              />
            </div>
            <div className="flex min-w-0 items-baseline gap-1.5 whitespace-nowrap">
              <span className="shrink-0 text-xs font-medium text-muted-foreground/80">
                首
              </span>
              <AuditLogTimestamp
                unixSeconds={row.first_seen_at}
                className={activityTimeClassName}
              />
            </div>
          </div>
          {spanLine}
        </div>
      )}
    </td>
  )
}

/** Session list: stacked 合计 / 输入·输出 / 缓存 — avoids one unreadable slash line. */
function AuditThreadSessionUsageCell({ row }: { row: AuditThreadListItem }) {
  const cache = row.total_cached_prompt_tokens ?? 0
  return (
    <div className="flex max-w-[13rem] flex-col items-end gap-0.5 py-0.5 text-right leading-tight">
      <span className="text-sm tabular-nums text-foreground">
        合计{' '}
        <span className="font-medium">
          {formatSessionTokenField(row.total_tokens)}
        </span>
      </span>
      <span className="text-[11px] tabular-nums text-muted-foreground">
        输入 {formatSessionTokenField(row.total_prompt_tokens)} · 输出{' '}
        {formatSessionTokenField(row.total_completion_tokens)}
      </span>
      {cache > 0 ? (
        <span
          className="max-w-[13rem] text-[11px] text-muted-foreground"
          title="提示词侧命中上下文缓存的 token 量；计费口径因上游而异，通常已计入「输入」"
        >
          缓存命中 {formatSessionTokenField(cache)}
        </span>
      ) : null}
    </div>
  )
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
  detailSheetOpen = false,
  onToggleDetail,
}: {
  row: AuditLogListItem
  /** 嵌套在会话抽屉内请求表时使用更紧凑的单元格（列与 v1 一致） */
  density?: 'default' | 'compact'
  /** 当前行是否正在右侧详情抽屉中展示 */
  detailSheetOpen?: boolean
  onToggleDetail: (requestId: string) => void
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
    <td className={cn(pad, 'w-12 text-center')}>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-pressed={detailSheetOpen}
            aria-label={
              detailSheetOpen ? '收起请求详情' : '查看请求详情'
            }
            className={cn(
              'inline-flex text-primary',
              detailSheetOpen && 'bg-muted/50',
            )}
            onClick={() => onToggleDetail(row.request_id)}
          >
            <FileText className="size-4" aria-hidden />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="left">
          {detailSheetOpen ? '收起' : '详情'}
        </TooltipContent>
      </Tooltip>
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

function V2ThreadRequestListPanel({
  isLoading,
  isError,
  data,
  activeDetailRequestId,
  onToggleDetail,
}: {
  isLoading: boolean
  isError: boolean
  data?: { data: AuditLogListItem[]; total: number }
  activeDetailRequestId: string | null
  onToggleDetail: (requestId: string) => void
}) {
  if (isLoading) {
    return <p className="text-xs text-muted-foreground">加载中…</p>
  }
  if (isError) {
    return (
      <p className="text-xs text-red-600" role="alert">
        请求列表加载失败，请稍后重试。
      </p>
    )
  }
  if (!data?.data.length) {
    return (
      <p className="text-xs text-muted-foreground">当前筛选下暂无请求。</p>
    )
  }
  return (
    <>
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
                  <th
                    scope="col"
                    className="w-10 px-2 py-1.5 text-center font-medium"
                  >
                    <span className="sr-only">操作</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {data.data.map((r) => (
                  <AuditLogTableRow
                    key={r.request_id}
                    row={r}
                    density="compact"
                    detailSheetOpen={activeDetailRequestId === r.request_id}
                    onToggleDetail={onToggleDetail}
                  />
                ))}
              </tbody>
            </table>
          </div>
        </TooltipProvider>
      </div>
      {data.total > THREAD_CHILD_LIMIT ? (
        <p className="mt-2 text-[11px] text-muted-foreground">
          仅显示前 {THREAD_CHILD_LIMIT} 条，共 {data.total}{' '}
          条；完整列表请使用底部「经典列表」或行内列表图标打开经典视图。
        </p>
      ) : null}
    </>
  )
}

function AuditThreadTableGroup({
  row,
  requestListHref,
  expanded,
  onToggleExpand,
}: {
  row: AuditThreadListItem
  requestListHref: string
  expanded: boolean
  onToggleExpand: () => void
}) {
  return (
    <tr
      className={cn(
        'border-b border-border/80 hover:bg-muted/30',
        expanded && 'bg-muted/25',
      )}
    >
      <td className="w-12 px-2 py-2 text-center align-middle">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-pressed={expanded}
              aria-label={
                expanded ? '收起会话下的请求' : '查看会话下的请求'
              }
              className={cn(
                'inline-flex text-primary',
                expanded && 'bg-muted/50',
              )}
              onClick={onToggleExpand}
            >
              <FileText className="size-4" aria-hidden />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="right">
            {expanded ? '收起' : '会话下的请求'}
          </TooltipContent>
        </Tooltip>
      </td>
      <AuditThreadActivityCell row={row} />
      <td
        className="max-w-[min(100vw,24rem)] truncate px-3 py-2 font-mono text-xs"
        title={row.thread_id}
      >
        {row.thread_id}
      </td>
      <td className="px-3 py-2 text-right tabular-nums">{row.request_count}</td>
      <td
        className="px-3 py-2 text-right tabular-nums"
        title="各次请求 latency_ms 之和；展示为 ms / 秒 / 分 / 时 等"
      >
        {formatCumulativeLatencyMs(row.total_latency_ms)}
      </td>
      <td className="max-w-[14rem] px-3 py-2 align-top">
        <AuditThreadSessionUsageCell row={row} />
      </td>
      <td className="px-3 py-2 text-right tabular-nums">
        {formatCostUsd(row.total_cost)}
      </td>
      <td className="px-3 py-2 text-right tabular-nums text-muted-foreground">
        {row.error_count > 0 ? (
          <span className="text-destructive">{row.error_count}</span>
        ) : (
          '0'
        )}
      </td>
      <td className="px-3 py-2 text-center">
        <Tooltip>
          <TooltipTrigger asChild>
            <Link
              to={requestListHref}
              aria-label="在经典列表中查看该会话的请求"
              className={cn(
                buttonVariants({ variant: 'ghost', size: 'icon-sm' }),
                'inline-flex text-primary',
              )}
            >
              <List className="size-4" aria-hidden />
            </Link>
          </TooltipTrigger>
          <TooltipContent side="left">请求列表</TooltipContent>
        </Tooltip>
      </td>
    </tr>
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
  /** v2: 右侧抽屉展示该会话下的请求列表（同时只打开一个） */
  const [expandedThreadId, setExpandedThreadId] = useState<string | null>(null)
  const [detailRequestId, setDetailRequestId] = useState<string | null>(null)

  const toggleLogDetail = useCallback((requestId: string) => {
    setDetailRequestId((prev) => (prev === requestId ? null : requestId))
  }, [])

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

  const threadSheetListQuery = useMemo(
    () =>
      auditLogListQuery({
        startTime,
        endTime,
        limit: THREAD_CHILD_LIMIT,
        offset: 0,
        keyword,
        model,
        appId,
        threadId: expandedThreadId ?? '',
        finishReason,
        statusCode,
        tokenId,
      }),
    [
      startTime,
      endTime,
      expandedThreadId,
      keyword,
      model,
      appId,
      finishReason,
      statusCode,
      tokenId,
    ],
  )

  const threadSheetChildList = useAuditLogList(threadSheetListQuery, {
    enabled: listVariant === 'v2' && expandedThreadId != null,
  })

  const listV1 = useAuditLogList(listQuery, { enabled: listVariant === 'v1' })
  const listV2 = useAuditThreadList(listQuery, { enabled: listVariant === 'v2' })
  useEffect(() => {
    if (listVariant !== 'v2') {
      setExpandedThreadId((id) => (id ? null : id))
      return
    }
    if (!listV2.data?.data.length) return
    const ids = new Set(listV2.data.data.map((r) => r.thread_id))
    if (expandedThreadId && !ids.has(expandedThreadId)) {
      setExpandedThreadId(null)
    }
  }, [listVariant, listV2.data, expandedThreadId])

  const data = listVariant === 'v1' ? listV1.data : listV2.data
  const isLoading = listVariant === 'v1' ? listV1.isLoading : listV2.isLoading
  const isError = listVariant === 'v1' ? listV1.isError : listV2.isError
  const isFetching = listVariant === 'v1' ? listV1.isFetching : listV2.isFetching
  /** Background refetch (e.g. WS thread rollup) should not spin the header refresh icon. */
  const listHasCachedData = Boolean(data)
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
  }, [listV1, listV2])

  const listFetchSpin =
    manualRefreshSpin || (isFetching && !listHasCachedData)

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
              ? '每行一个会话 (thread)：筛选与经典列表一致；点击行首文档图标在右侧打开该会话下的请求列表（最多 50 条），与经典列表中查看请求详情相同。请求数为维表累计次数。'
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

      {!isLoading &&
        listVariant === 'v1' &&
        listV1.data &&
        listV1.data.data.length > 0 && (
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
                <th
                  scope="col"
                  className="w-12 px-3 py-2 text-center font-medium"
                >
                  <span className="sr-only">操作</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {listV1.data.data.map((row) => (
                <AuditLogTableRow
                  key={row.request_id}
                  row={row}
                  detailSheetOpen={detailRequestId === row.request_id}
                  onToggleDetail={toggleLogDetail}
                />
              ))}
            </tbody>
          </table>
          </div>
        </TooltipProvider>
      )}

      {!isLoading && listV2.data && listV2.data.data.length > 0 && listVariant === 'v2' && (
        <TooltipProvider delayDuration={250} skipDelayDuration={200}>
        <div className="overflow-x-auto rounded-lg border border-border">
          <table className="w-full min-w-[1100px] border-collapse text-left text-sm">
            <thead className="border-b border-border bg-muted/40">
              <tr>
                <th
                  scope="col"
                  className="w-12 px-2 py-2 text-center font-medium"
                >
                  <span className="sr-only">会话下的请求</span>
                </th>
                <th
                  scope="col"
                  className="px-3 py-2 font-medium"
                  title="末次与首次请求时间，及二者墙钟间隔（跨度）；列表按末次排序"
                >
                  活动时间
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  会话 (thread)
                </th>
                <th scope="col" className="px-3 py-2 text-right font-medium">
                  请求数
                </th>
                <th
                  scope="col"
                  className="px-3 py-2 text-right font-medium"
                  title="本会话下各次请求的 latency_ms 之和（缺失计 0）；自动使用 ms / 秒 / 分 / 时 / 天"
                >
                  累计耗时
                </th>
                <th
                  scope="col"
                  className="min-w-[10rem] px-3 py-2 text-right font-medium"
                  title="当前筛选范围内汇总：合计 token、输入/输出拆分；若有缓存命中会单独标注"
                >
                  用量
                </th>
                <th scope="col" className="px-3 py-2 text-right font-medium">
                  成本 (USD)
                </th>
                <th
                  scope="col"
                  className="px-3 py-2 text-right font-medium"
                  title="HTTP status ≥ 400"
                >
                  失败
                </th>
                <th
                  scope="col"
                  className="w-12 px-3 py-2 text-center font-medium"
                >
                  <span className="sr-only">操作</span>
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
                  onToggleExpand={() => {
                    setDetailRequestId(null)
                    setExpandedThreadId((cur) =>
                      cur === row.thread_id ? null : row.thread_id,
                    )
                  }}
                />
              ))}
            </tbody>
          </table>
        </div>
        </TooltipProvider>
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

    <Sheet
      open={expandedThreadId != null}
      onOpenChange={(open) => {
        if (!open) setExpandedThreadId(null)
      }}
    >
      <SheetContent
        side="right"
        className="flex h-full max-h-[100dvh] flex-col gap-0 overflow-hidden p-0"
        style={logSheetPanelStyle}
      >
        <SheetHeader className="border-border shrink-0 space-y-1 border-b px-4 py-3 text-left">
          <SheetTitle>会话下的请求</SheetTitle>
          <SheetDescription className="sr-only">
            当前筛选下该 thread 的请求行，最多 {THREAD_CHILD_LIMIT} 条
          </SheetDescription>
          {expandedThreadId ? (
            <p className="break-all font-mono text-xs text-muted-foreground">
              {expandedThreadId}
            </p>
          ) : null}
        </SheetHeader>
        <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
          <V2ThreadRequestListPanel
            isLoading={threadSheetChildList.isLoading}
            isError={threadSheetChildList.isError}
            data={threadSheetChildList.data}
            activeDetailRequestId={detailRequestId}
            onToggleDetail={toggleLogDetail}
          />
        </div>
        <SheetFooter className="border-border shrink-0 border-t px-4 py-3 sm:flex-row sm:justify-end">
          {expandedThreadId ? (
            <Button variant="outline" size="sm" asChild>
              <Link
                to={hrefToClassicRequestListForThread(
                  expandedThreadId,
                  searchParams,
                )}
              >
                在经典列表中打开
              </Link>
            </Button>
          ) : null}
        </SheetFooter>
      </SheetContent>
    </Sheet>

    <Sheet
      open={detailRequestId != null}
      onOpenChange={(open) => {
        if (!open) setDetailRequestId(null)
      }}
    >
      <SheetContent
        side="right"
        className="flex h-full max-h-[100dvh] flex-col gap-0 overflow-hidden p-0"
        style={logSheetPanelStyle}
      >
        <SheetHeader className="border-border shrink-0 border-b px-4 py-3 text-left">
          <SheetTitle>日志详情</SheetTitle>
          <SheetDescription className="sr-only">
            当前请求的审计字段、头、正文与 metadata
          </SheetDescription>
        </SheetHeader>
        <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
          {detailRequestId ? (
            <LogDetailContent requestId={detailRequestId} />
          ) : null}
        </div>
        <SheetFooter className="border-border shrink-0 border-t px-4 py-3 sm:flex-row sm:justify-end">
          {detailRequestId ? (
            <Button variant="outline" size="sm" asChild>
              <Link
                to={`/logs/${encodeURIComponent(detailRequestId)}`}
                target="_blank"
                rel="noreferrer"
              >
                新标签页打开
              </Link>
            </Button>
          ) : null}
        </SheetFooter>
      </SheetContent>
    </Sheet>
    </TooltipProvider>
  )
}
