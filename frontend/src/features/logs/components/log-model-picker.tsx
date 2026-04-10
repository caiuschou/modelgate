import { useId, useMemo, useRef, useState } from 'react'
import { ChevronDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { readRecentLogModels } from '@/features/logs/log-recent-models'
import { buildLogModelSuggestions, LOG_MODEL_SUGGEST_CAP } from '@/features/logs/log-model-suggestions'
import { useOpenRouterModels } from '@/features/models/hooks/use-openrouter-models'
import { cn } from '@/lib/utils'

type LogModelPickerProps = {
  id: string
  value: string
  onChange: (next: string) => void
  /** Bump when storage may have changed (e.g. after successful query). */
  storageRevision: number
}

export function LogModelPicker({
  id,
  value,
  onChange,
  storageRevision,
}: LogModelPickerProps) {
  const listId = useId()
  const fieldRef = useRef<HTMLDivElement>(null)
  const [open, setOpen] = useState(false)
  const { data: catalog, isPending, isError } = useOpenRouterModels()

  /** 输入框与触发器在 Content 外，需视为「组合控件」以免点输入时被判定为外部点击而关闭 */
  const isOutsideButInsideField = (target: EventTarget | null) =>
    target instanceof Node && Boolean(fieldRef.current?.contains(target))

  const recent = useMemo(() => {
    void storageRevision
    return readRecentLogModels()
  }, [storageRevision])

  const pack = useMemo(
    () => buildLogModelSuggestions(catalog, recent, value),
    [catalog, recent, value],
  )

  const showUseCustom = pack.showUseCustom

  const pick = (name: string) => {
    onChange(name)
    setOpen(false)
  }

  return (
    <div ref={fieldRef} className="relative mt-1">
      <Input
        id={id}
        name="log-model"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onFocus={() => setOpen(true)}
        placeholder="精确匹配模型名（输入时从全目录筛选，应用查询后生效）"
        className="pr-10 font-mono text-sm"
        autoComplete="off"
      />
      <Popover
        open={open}
        onOpenChange={setOpen}
      >
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className="absolute right-0.5 top-1/2 size-7 -translate-y-1/2 shrink-0 text-muted-foreground"
            aria-label="展开模型列表（OpenRouter 全目录 + 最近使用排序）"
            aria-expanded={open}
            aria-haspopup="listbox"
            aria-controls={open ? listId : undefined}
          >
            <ChevronDown className="size-4" aria-hidden />
          </Button>
        </PopoverTrigger>
        <PopoverContent
          id={listId}
          align="end"
          className="w-[min(calc(100vw-2rem),28rem)] p-2"
          sideOffset={6}
          onOpenAutoFocus={(e) => e.preventDefault()}
          onPointerDownOutside={(e) => {
            if (isOutsideButInsideField(e.target)) e.preventDefault()
          }}
          onInteractOutside={(e) => {
            if (isOutsideButInsideField(e.target)) e.preventDefault()
          }}
          onFocusOutside={(e) => {
            if (isOutsideButInsideField(e.target)) e.preventDefault()
          }}
        >
          <p className="mb-2 px-0.5 text-xs text-muted-foreground">
            {value.trim() === ''
              ? '以下为最近使用；输入文字可从 OpenRouter 全目录筛选，结果按最近使用时间排序。'
              : '以下从全目录筛选，已按最近使用时间排序（未用过的靠后）。'}
          </p>

          {isPending && value.trim() !== '' && (
            <p className="px-1 py-2 text-sm text-muted-foreground">正在加载模型目录…</p>
          )}
          {isError && value.trim() !== '' && (
            <p className="px-1 py-2 text-sm text-amber-800 dark:text-amber-300">
              目录加载失败，请直接输入完整模型 id。
            </p>
          )}

          {value.trim() === '' &&
            recent.length === 0 &&
            !showUseCustom && (
              <p className="px-1 py-4 text-center text-sm text-muted-foreground">
                暂无最近记录。输入关键字即可搜索全部模型。
              </p>
            )}

          {showUseCustom && (
            <button
              type="button"
              className="w-full rounded-md border border-dashed border-border px-2 py-2 text-left text-sm text-muted-foreground hover:bg-muted"
              onClick={() => pick(value.trim())}
            >
              使用「<span className="font-mono text-foreground">{value.trim()}</span>」
            </button>
          )}

          {!showUseCustom && pack.items.length > 0 && (
            <ul
              role="listbox"
              className="max-h-60 space-y-0.5 overflow-y-auto overscroll-contain py-0.5"
            >
              {pack.items.map((item) =>
                item.kind === 'recent_only' ? (
                  <li key={`r:${item.name}`} role="presentation">
                    <button
                      type="button"
                      role="option"
                      aria-selected={value.trim() === item.name}
                      className={cn(
                        'flex w-full flex-col gap-0.5 rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted',
                        value.trim() === item.name && 'bg-muted/80',
                      )}
                      onClick={() => pick(item.name)}
                    >
                      <span className="font-mono text-xs leading-tight">{item.name}</span>
                      <span className="text-[10px] text-muted-foreground">
                        最近使用 {new Date(item.lastUsedAt).toLocaleString()}
                      </span>
                    </button>
                  </li>
                ) : (
                  <li key={`c:${item.row.id}`} role="presentation">
                    <button
                      type="button"
                      role="option"
                      aria-selected={value.trim() === item.row.id}
                      className={cn(
                        'flex w-full flex-col gap-0.5 rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted',
                        value.trim() === item.row.id && 'bg-muted/80',
                      )}
                      onClick={() => pick(item.row.id)}
                    >
                      <span className="font-mono text-xs leading-tight">{item.row.id}</span>
                      {item.row.name && item.row.name !== item.row.id && (
                        <span className="text-[11px] text-muted-foreground">{item.row.name}</span>
                      )}
                      <span className="text-[10px] text-muted-foreground">
                        {item.lastUsedAt != null
                          ? `最近使用 ${new Date(item.lastUsedAt).toLocaleString()}`
                          : '尚未在本地记录使用'}
                      </span>
                    </button>
                  </li>
                ),
              )}
            </ul>
          )}

          {pack.catalogTruncated && (
            <p className="mt-2 px-0.5 text-[11px] text-muted-foreground">
              仅显示前 {LOG_MODEL_SUGGEST_CAP} 条，请缩小筛选词。
            </p>
          )}
        </PopoverContent>
      </Popover>
    </div>
  )
}
