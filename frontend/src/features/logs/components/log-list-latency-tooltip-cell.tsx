import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { formatLatencySeconds } from '@/features/logs/format-latency'
import type { AuditLogListItem } from '@/features/logs/types'
import { cn } from '@/lib/utils'

function RowLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="shrink-0 text-muted-foreground">{label}</span>
      <span className="min-w-0 text-right font-mono">{value}</span>
    </div>
  )
}

function secLabel(formatted: string): string {
  return formatted === '—' ? '—' : `${formatted} s`
}

export function LogListLatencyTooltipCell({
  row,
  children,
}: {
  row: AuditLogListItem
  children: React.ReactNode
}) {
  const total = secLabel(formatLatencySeconds(row.latency_ms))
  const reasoning = secLabel(formatLatencySeconds(row.reasoning_phase_ms))
  const hasReasoning =
    row.reasoning_phase_ms != null && Number.isFinite(row.reasoning_phase_ms)

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          className={cn(
            'cursor-default border-b border-dotted border-muted-foreground/50',
          )}
        >
          {children}
        </span>
      </TooltipTrigger>
      <TooltipContent side="left" className="max-w-xs text-xs">
        <div className="flex flex-col gap-1.5">
          <RowLine label="总耗时（端到端）" value={total} />
          {hasReasoning ? (
            <RowLine label="推理→首条正文" value={reasoning} />
          ) : null}
          <p className="text-[10px] leading-snug text-muted-foreground">
            {hasReasoning
              ? '「推理→首条正文」为流式响应内，从首次出现推理输出到首次出现正文的时间。'
              : '无流式推理阶段拆分（非流式或未出现推理字段）。'}
          </p>
        </div>
      </TooltipContent>
    </Tooltip>
  )
}
