import type { CSSProperties } from 'react'
import { Copy } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  formatContextShort,
  formatCreatedLabel,
  formatInputPricePerM,
} from '@/features/models/lib/model-explorer-logic'
import { openRouterModelHref } from '@/features/models/lib/openrouter-links'
import type { OpenRouterModelRow } from '@/features/models/lib/openrouter-models'
import { cn } from '@/lib/utils'

export function ModelsExplorerListRow({
  model: m,
  style,
  onCopy,
  compact = false,
}: {
  model: OpenRouterModelRow
  style?: CSSProperties
  onCopy: () => void
  compact?: boolean
}) {
  const wrapClass = style
    ? 'absolute left-0 top-0 w-full border-b border-border/70 px-1 py-1.5'
    : 'border-b border-border/70 px-2 py-2'

  return (
    <div className={wrapClass} style={style} role="listitem">
      <div className="flex gap-2">
        <div className="min-w-0 flex-1">
          <a
            href={openRouterModelHref(m.id)}
            target="_blank"
            rel="noreferrer"
            className={cn(
              'line-clamp-1 font-medium text-foreground hover:underline',
              compact ? 'text-sm' : 'text-base',
            )}
          >
            {m.name ?? m.id}
          </a>
          <p
            className={cn(
              'text-muted-foreground',
              compact ? 'line-clamp-1 text-xs' : 'line-clamp-2 text-sm',
            )}
          >
            {m.description ?? '—'}
          </p>
          <p className="mt-0.5 text-[11px] text-muted-foreground">
            <code className="rounded bg-muted/80 px-1">{m.id}</code>
            {m.created ? (
              <span className="ml-2">{formatCreatedLabel(m.created)}</span>
            ) : null}
            <span className="ml-2">{formatContextShort(m.context_length)}</span>
            <span className="ml-2">
              {formatInputPricePerM(m.pricing?.prompt)}
            </span>
          </p>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="shrink-0"
          title="复制模型 id"
          aria-label={`复制 ${m.id}`}
          onClick={onCopy}
        >
          <Copy className={compact ? 'size-3.5' : 'size-4'} />
        </Button>
      </div>
    </div>
  )
}

export function ModelsExplorerGridCard({
  model: m,
  onCopy,
}: {
  model: OpenRouterModelRow
  onCopy: (id: string) => void
}) {
  return (
    <div
      role="listitem"
      className="flex flex-col rounded-lg border border-border bg-card p-3 text-left shadow-sm"
    >
      <a
        href={openRouterModelHref(m.id)}
        target="_blank"
        rel="noreferrer"
        className="line-clamp-2 text-sm font-medium leading-snug hover:underline"
      >
        {m.name ?? m.id}
      </a>
      <code className="mt-1 truncate text-xs text-muted-foreground">{m.id}</code>
      <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">
        {m.description ?? ''}
      </p>
      <div className="mt-auto flex items-center justify-between pt-2">
        <span className="text-xs text-muted-foreground">
          {formatContextShort(m.context_length)}
        </span>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="size-8"
          aria-label={`复制 ${m.id}`}
          onClick={() => onCopy(m.id)}
        >
          <Copy className="size-3.5" />
        </Button>
      </div>
    </div>
  )
}
