import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { useAuditLogBody, useAuditLogDetail } from '@/features/logs/hooks/use-logs'
import { parseUsageFromBody } from '@/features/logs/parse-log-usage'
import type { AuditLogListItem } from '@/features/logs/types'
import { formatCostUsd } from '@/lib/format-cost'
import { cn } from '@/lib/utils'

function numFmt(n: number | null | undefined): string {
  if (n === null || n === undefined) return '—'
  return n.toLocaleString()
}

function costFmt(v: unknown): string {
  if (typeof v === 'number' && Number.isFinite(v)) return formatCostUsd(v)
  if (typeof v === 'string') {
    const n = Number(v)
    if (Number.isFinite(n)) return formatCostUsd(n)
  }
  return '—'
}

function RowLine({
  label,
  value,
  mono,
}: {
  label: string
  value: string
  mono?: boolean
}) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="shrink-0 text-muted-foreground">{label}</span>
      <span className={cn('min-w-0 text-right', mono && 'font-mono')}>{value}</span>
    </div>
  )
}

function DetailLine({ label, value }: { label: string; value?: number }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      <span
        className={cn(
          'font-mono',
          value === 0 ? 'text-muted-foreground' : '',
        )}
      >
        {value?.toLocaleString() ?? '—'}
      </span>
    </div>
  )
}

export function LogListUsageTooltipCell({
  row,
  highlight,
  children,
}: {
  row: AuditLogListItem
  /** When set, emphasizes that section in the tooltip; omit for merged list cells. */
  highlight?: 'prompt' | 'completion'
  children: React.ReactNode
}) {
  const [open, setOpen] = useState(false)
  const detail = useAuditLogDetail(open ? row.request_id : undefined, {
    pollIncompleteStream: false,
  })
  const bodyEnabled = open && Boolean(detail.data?.response_body_path)
  const body = useAuditLogBody(
    open ? row.request_id : undefined,
    'response',
    bodyEnabled,
  )
  const usage = useMemo(
    () => (body.data ? parseUsageFromBody(body.data) : null),
    [body.data],
  )

  const loadingBreakdown =
    open &&
    (detail.isLoading ||
      (Boolean(detail.data?.response_body_path) && body.isLoading))

  const promptSection = (
    <div
      className={cn(
        'space-y-1 rounded-md border border-transparent p-1.5',
        highlight === 'prompt' &&
          'border-sky-500/50 bg-sky-500/5',
      )}
    >
      <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        输入（prompt）
      </p>
      <RowLine label="Tokens" value={numFmt(row.prompt_tokens)} mono />
      <RowLine
        label="缓存命中"
        value={numFmt(row.cached_prompt_tokens)}
        mono
      />
      {usage?.prompt_tokens_details ? (
        <div className="mt-1 space-y-0.5 border-t border-border/60 pt-1">
          <DetailLine
            label="明细 · 缓存命中"
            value={usage.prompt_tokens_details.cached_tokens}
          />
          <DetailLine
            label="明细 · 缓存写入"
            value={usage.prompt_tokens_details.cache_write_tokens}
          />
          <DetailLine
            label="明细 · 音频"
            value={usage.prompt_tokens_details.audio_tokens}
          />
          <DetailLine
            label="明细 · 视频"
            value={usage.prompt_tokens_details.video_tokens}
          />
        </div>
      ) : null}
    </div>
  )

  const completionSection = (
    <div
      className={cn(
        'space-y-1 rounded-md border border-transparent p-1.5',
        highlight === 'completion' &&
          'border-emerald-500/50 bg-emerald-500/5',
      )}
    >
      <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        输出（completion）
      </p>
      <RowLine label="Tokens" value={numFmt(row.completion_tokens)} mono />
      {usage?.completion_tokens_details ? (
        <div className="mt-1 space-y-0.5 border-t border-border/60 pt-1">
          <DetailLine
            label="明细 · 推理"
            value={usage.completion_tokens_details.reasoning_tokens}
          />
          <DetailLine
            label="明细 · 图片"
            value={usage.completion_tokens_details.image_tokens}
          />
          <DetailLine
            label="明细 · 音频"
            value={usage.completion_tokens_details.audio_tokens}
          />
        </div>
      ) : null}
    </div>
  )

  const costSection =
    row.cost != null ||
    usage?.cost != null ||
    usage?.cost_details != null ? (
      <div className="space-y-1 rounded-md border border-border/60 p-1.5">
        <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
          成本
        </p>
        <RowLine
          label="审计表 / 合计"
          value={row.cost != null ? formatCostUsd(row.cost) : '—'}
          mono
        />
        {usage?.cost != null ? (
          <RowLine label="响应 usage · 总成本" value={costFmt(usage.cost)} mono />
        ) : null}
        {usage?.cost_details ? (
          <div className="mt-1 space-y-0.5 border-t border-border/60 pt-1">
            <RowLine
              label="上游推理"
              value={costFmt(usage.cost_details.upstream_inference_cost)}
              mono
            />
            <RowLine
              label="其中 · 输入"
              value={costFmt(usage.cost_details.upstream_inference_prompt_cost)}
              mono
            />
            <RowLine
              label="其中 · 输出"
              value={costFmt(
                usage.cost_details.upstream_inference_completions_cost,
              )}
              mono
            />
          </div>
        ) : null}
      </div>
    ) : null

  return (
    <Tooltip
      onOpenChange={(next) => {
        setOpen(next)
      }}
    >
      <TooltipTrigger asChild>
        <div
          className="w-full cursor-help border-b border-dotted border-muted-foreground/40 text-right outline-none"
          tabIndex={0}
        >
          {children}
        </div>
      </TooltipTrigger>
      <TooltipContent
        side="left"
        align="center"
        className="w-[min(100vw-2rem,26rem)] max-w-none overflow-visible p-3 text-xs"
      >
        <p className="mb-2 font-medium leading-snug">用量与计费</p>
        {loadingBreakdown ? (
          <p className="text-muted-foreground">正在加载响应明细…</p>
        ) : null}
        {detail.data && !detail.data.response_body_path && !detail.isLoading ? (
          <p className="mb-2 text-[11px] text-muted-foreground">
            无响应正文文件时，仅显示审计表中的汇总字段；细分依赖上游返回的
            usage。
          </p>
        ) : null}
        {body.isError ? (
          <p className="mb-2 text-[11px] text-amber-800 dark:text-amber-200">
            加载响应正文失败，明细暂不可用（请稍后重试）。
          </p>
        ) : null}
        {body.isSuccess &&
        body.data === null &&
        detail.data?.response_body_path &&
        !detail.isLoading ? (
          <p className="mb-2 text-[11px] text-muted-foreground">
            响应正文文件不可用（已清理或路径失效），以下为审计表汇总。
          </p>
        ) : null}
        <div className="space-y-2">
          {promptSection}
          {completionSection}
          <div className="space-y-1 rounded-md border border-border/60 p-1.5">
            <RowLine label="合计 Tokens" value={numFmt(row.total_tokens)} mono />
          </div>
          {costSection}
        </div>
        <div className="mt-3 border-t border-border/60 pt-2">
          <Link
            to={`/logs/${encodeURIComponent(row.request_id)}`}
            className="text-primary underline-offset-4 hover:underline"
            onClick={() => setOpen(false)}
          >
            打开详情页
          </Link>
        </div>
      </TooltipContent>
    </Tooltip>
  )
}
