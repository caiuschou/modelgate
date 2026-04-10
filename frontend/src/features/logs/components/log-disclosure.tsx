import type { ReactNode } from 'react'
import { ChevronRight } from 'lucide-react'
import { cn } from '@/lib/utils'

/** Parent `<details>` must include `className={logDetailsGroupClass(name)}`. */
export function logDetailsGroupClass(name: string, ...extra: string[]): string {
  return cn(`group/${name} min-w-0`, ...extra)
}

export function LogDetailsSummary({
  groupName,
  className,
  children,
}: {
  /** Matches `logDetailsGroupClass(groupName)` on the parent `<details>`. */
  groupName: string
  className?: string
  children: ReactNode
}) {
  return (
    <summary
      className={cn(
        'flex cursor-pointer list-none items-center gap-2 [&::-webkit-details-marker]:hidden',
        className,
      )}
    >
      <ChevronRight
        className={cn(
          'size-4 shrink-0 text-muted-foreground transition-transform duration-200',
          `group-open/${groupName}:rotate-90`,
        )}
        aria-hidden
      />
      {children}
    </summary>
  )
}
