import { useId, useState } from 'react'
import { endOfDay, format, startOfDay } from 'date-fns'
import { zhCN } from 'date-fns/locale'
import { CalendarIcon } from 'lucide-react'
import { Calendar } from '@/components/ui/calendar'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { cn } from '@/lib/utils'

type LogDateFieldMode = 'start' | 'end'

function unixSecondsToDate(ts: number): Date {
  return new Date(ts * 1000)
}

function startOfLocalDayUnix(d: Date): number {
  return Math.floor(startOfDay(d).getTime() / 1000)
}

function endOfLocalDayUnix(d: Date): number {
  return Math.floor(endOfDay(d).getTime() / 1000)
}

export function LogDateField({
  label,
  valueUnix,
  mode,
  onChangeUnix,
}: {
  label: string
  valueUnix: number
  mode: LogDateFieldMode
  onChangeUnix: (unix: number) => void
}) {
  const [open, setOpen] = useState(false)
  const labelId = useId()
  const day = startOfDay(unixSecondsToDate(valueUnix))

  return (
    <div className="text-sm">
      <span className="text-muted-foreground" id={labelId}>
        {label}
      </span>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="outline"
            aria-labelledby={labelId}
            className={cn(
              'mt-1 h-8 w-full justify-start px-2.5 text-left font-normal',
            )}
          >
            <CalendarIcon className="mr-2 size-4 opacity-70" aria-hidden />
            {format(day, 'yyyy年M月d日', { locale: zhCN })}
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
