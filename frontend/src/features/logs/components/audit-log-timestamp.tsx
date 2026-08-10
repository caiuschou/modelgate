import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import {
  formatLogTimestampHuman,
  formatLogTimestampRaw,
} from '@/features/logs/format-log-timestamp'

export function AuditLogTimestamp({
  unixSeconds,
  className,
}: {
  unixSeconds: number
  className?: string
}) {
  const human = formatLogTimestampHuman(unixSeconds)
  const raw = formatLogTimestampRaw(unixSeconds)
  const iso = new Date(unixSeconds * 1000).toISOString()

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <time
          dateTime={iso}
          className={
            className ??
            'cursor-default underline decoration-dotted decoration-muted-foreground/50 underline-offset-2'
          }
        >
          {human}
        </time>
      </TooltipTrigger>
      <TooltipContent side="top" surface="panel" className="max-w-sm">
        <p className="text-[11px] text-muted-foreground">原始时间</p>
        <p className="font-mono text-xs">{raw}</p>
      </TooltipContent>
    </Tooltip>
  )
}
