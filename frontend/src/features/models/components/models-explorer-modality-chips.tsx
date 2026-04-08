import { cn } from '@/lib/utils'
import type { ModalityChipId } from '@/features/models/lib/model-explorer-logic'
import { MODALITY_CHIPS } from '@/features/models/lib/model-explorer-logic'

export function ModelsExplorerModalityChips({
  counts,
  value,
  onChange,
  size = 'md',
}: {
  counts: Record<Exclude<ModalityChipId, 'all'>, number>
  value: ModalityChipId
  onChange: (id: ModalityChipId) => void
  size?: 'sm' | 'md'
}) {
  const pad = size === 'sm' ? 'px-2 py-1 text-xs' : 'px-2.5 py-1.5 text-xs'
  return (
    <div
      className="flex flex-wrap gap-1.5"
      role="tablist"
      aria-label="按输出类型筛选"
    >
      <button
        type="button"
        role="tab"
        aria-selected={value === 'all'}
        className={cn(
          'rounded-full border font-medium transition-colors',
          pad,
          value === 'all'
            ? 'border-primary bg-primary text-primary-foreground'
            : 'border-border bg-background text-muted-foreground hover:bg-accent hover:text-foreground',
        )}
        onClick={() => onChange('all')}
      >
        全部
      </button>
      {MODALITY_CHIPS.map(({ id, label }) => (
        <button
          key={id}
          type="button"
          role="tab"
          aria-selected={value === id}
          className={cn(
            'rounded-full border font-medium transition-colors',
            pad,
            value === id
              ? 'border-primary bg-primary text-primary-foreground'
              : 'border-border bg-background text-muted-foreground hover:bg-accent hover:text-foreground',
          )}
          onClick={() => onChange(id)}
        >
          {label} ({counts[id]})
        </button>
      ))}
    </div>
  )
}
