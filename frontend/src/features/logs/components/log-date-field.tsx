import { useId, useState } from 'react'
import { format, startOfDay } from 'date-fns'
import { zhCN } from 'date-fns/locale'
import { CalendarIcon } from 'lucide-react'
import { Calendar } from '@/components/ui/calendar'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { endOfLocalDayUnix, startOfLocalDayUnix } from '@/features/logs/log-list-range'
import { cn } from '@/lib/utils'

type LogDateFieldMode = 'start' | 'end'

function unixSecondsToDate(ts: number): Date {
  return new Date(ts * 1000)
}

export function LogDateField({
  label,
  valueUnix,
  mode,
  onChangeUnix,
  compact = false,
}: {
  label: string
  valueUnix: number
  mode: LogDateFieldMode
  onChangeUnix: (unix: number) => void
  /** Tighter typography and shorter date label for dense filter bars. */
  compact?: boolean
}) {
  const [open, setOpen] = useState(false)
  const labelId = useId()
  const day = startOfDay(unixSecondsToDate(valueUnix))

  const dateLabel = compact
    ? format(day, 'yyyy/M/d', { locale: zhCN })
    : format(day, 'yyyy年M月d日', { locale: zhCN })

  return (
    <div className={cn(compact ? 'text-xs' : 'text-sm')}>
      <span className="text-muted-foreground" id={labelId}>
        {label}
      </span>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="outline"
            aria-labelledby={labelId}
            title={
              mode === 'start'
                ? '该日本地 0:00:00（日初）'
                : '该日本地 23:59:59.999（日末）'
            }
            className={cn(
              'mt-0.5 h-8 w-full justify-start px-2 text-left font-normal',
              compact && 'text-xs',
            )}
          >
            <CalendarIcon
              className={cn('mr-1.5 shrink-0 opacity-70', compact ? 'size-3.5' : 'size-4')}
              aria-hidden
            />
            {dateLabel}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto p-0" align="start">
          <Calendar
            mode="single"
            locale={zhCN}
            selected={day}
            defaultMonth={day}
            onSelect={(d) => {
              if (!d) return
              const unix =
                mode === 'start' ? startOfLocalDayUnix(d) : endOfLocalDayUnix(d)
              onChangeUnix(unix)
              setOpen(false)
            }}
          />
        </PopoverContent>
      </Popover>
    </div>
  )
}
