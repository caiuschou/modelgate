import { useVirtualizer } from '@tanstack/react-virtual'
import { useMemo, useRef } from 'react'

/** Line-based virtual scroll so large bodies do not mount one giant DOM node. */
export function VirtualizedLogBody({ text }: { text: string }) {
  const parentRef = useRef<HTMLDivElement>(null)
  const lines = useMemo(() => text.split('\n'), [text])
  const virtualizer = useVirtualizer({
    count: lines.length,
    getScrollElement: () => parentRef.current,
    // Initial guess only; each row height comes from measureElement (wrapped lines need >1 logical line).
    estimateSize: () => 20,
    overscan: 24,
  })

  return (
    <div
      ref={parentRef}
      className="max-h-[480px] overflow-auto rounded border border-border/60 bg-muted/50 font-mono text-xs"
    >
      <div
        className="relative w-full min-w-0"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualizer.getVirtualItems().map((vi) => (
          <div
            key={vi.key}
            data-index={vi.index}
            ref={virtualizer.measureElement}
            className="absolute left-0 top-0 w-full min-w-0 whitespace-pre-wrap break-words px-3 py-0.5"
            style={{ transform: `translateY(${vi.start}px)` }}
          >
            {lines[vi.index]}
          </div>
        ))}
      </div>
    </div>
  )
}
