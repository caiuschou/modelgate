import { useDeferredValue, useMemo, useRef, useState } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { LayoutGrid, LayoutList, Search } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/shared/empty-state'
import { ModelsExplorerModalityChips } from '@/features/models/components/models-explorer-modality-chips'
import {
  ModelsExplorerGridCard,
  ModelsExplorerListRow,
} from '@/features/models/components/models-explorer-items'
import { useOpenRouterModels } from '@/features/models/hooks/use-openrouter-models'
import type { ModalityChipId } from '@/features/models/lib/model-explorer-logic'
import {
  INPUT_MODALITY_OPTIONS,
  countByModality,
  filterByContextMin,
  filterByModality,
  filterByRequiredInputModalities,
  filterBySearch,
  filterBySeries,
  sortExplorerRows,
  type ExplorerSort,
  type SeriesId,
} from '@/features/models/lib/model-explorer-logic'

const LIST_ROW_H = 118
const LIST_MAX_H = 'min(70vh,720px)'
const GRID_PREVIEW_CAP = 120
const OVERSCAN = 12

type ViewMode = 'list' | 'grid'

const SERIES_OPTIONS: { id: SeriesId; label: string }[] = [
  { id: 'all', label: '全部' },
  { id: 'gpt', label: 'GPT' },
  { id: 'claude', label: 'Claude' },
  { id: 'gemini', label: 'Gemini' },
  { id: 'other', label: '其它' },
]

export function ModelsCatalogPage() {
  const { data, isPending, isError, error, refetch, isFetching } =
    useOpenRouterModels()
  const [query, setQuery] = useState('')
  const deferredQ = useDeferredValue(query)
  const [modality, setModality] = useState<ModalityChipId>('all')
  const [sort, setSort] = useState<ExplorerSort>('newest')
  const [view, setView] = useState<ViewMode>('list')
  const [series, setSeries] = useState<SeriesId>('all')
  const [ctxMinRaw, setCtxMinRaw] = useState('')
  const [requiredInputs, setRequiredInputs] = useState<Set<string>>(() => new Set())

  const ctxMin = useMemo(() => {
    const n = Number(ctxMinRaw.trim())
    return Number.isFinite(n) && n > 0 ? n : null
  }, [ctxMinRaw])

  const counts = useMemo(
    () => countByModality(data ?? []),
    [data],
  )

  const filtered = useMemo(() => {
    let rows = data ?? []
    rows = filterBySearch(rows, deferredQ)
    rows = filterByModality(rows, modality)
    rows = filterBySeries(rows, series)
    rows = filterByContextMin(rows, ctxMin)
    rows = filterByRequiredInputModalities(rows, requiredInputs)
    return sortExplorerRows(rows, sort)
  }, [
    data,
    deferredQ,
    modality,
    series,
    ctxMin,
    requiredInputs,
    sort,
  ])

  const parentRef = useRef<HTMLDivElement>(null)
  // eslint-disable-next-line react-hooks/incompatible-library -- virtualization row measurement API
  const virtualizer = useVirtualizer({
    count: view === 'list' ? filtered.length : 0,
    getScrollElement: () => parentRef.current,
    estimateSize: () => LIST_ROW_H,
    overscan: OVERSCAN,
  })

  const copyId = async (id: string) => {
    try {
      await navigator.clipboard.writeText(id)
    } catch {
      /* ignore */
    }
  }

  const toggleInputFilter = (id: string) => {
    setRequiredInputs((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  return (
    <div className="flex flex-col gap-6 pb-4 lg:flex-row lg:items-start">
      <aside className="w-full shrink-0 space-y-4 lg:w-52">
        <Card className="p-3">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Input Modalities
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            须同时支持所选输入类型
          </p>
          <ul className="mt-3 space-y-2">
            {INPUT_MODALITY_OPTIONS.map(({ id, label }) => (
              <li key={id}>
                <label className="flex cursor-pointer items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    className="size-4 rounded border-input"
                    checked={requiredInputs.has(id)}
                    onChange={() => toggleInputFilter(id)}
                  />
                  {label}
                </label>
              </li>
            ))}
          </ul>
        </Card>
        <Card className="p-3">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Context Length
          </h2>
          <label className="mt-2 block text-sm">
            <span className="text-muted-foreground">最小上下文（tokens）</span>
            <Input
              className="mt-1"
              inputMode="numeric"
              placeholder="例如 8192"
              value={ctxMinRaw}
              onChange={(e) => setCtxMinRaw(e.target.value)}
              aria-label="最小上下文长度"
            />
          </label>
        </Card>
        <Card className="p-3">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Series
          </h2>
          <div className="mt-2 flex flex-col gap-1">
            {SERIES_OPTIONS.map(({ id, label }) => (
              <Button
                key={id}
                type="button"
                variant={series === id ? 'secondary' : 'ghost'}
                size="sm"
                className="h-8 justify-start font-normal"
                onClick={() => setSeries(id)}
              >
                {label}
              </Button>
            ))}
          </div>
        </Card>
      </aside>

      <div className="min-w-0 flex-1 space-y-4">
        <div>
          <h1 className="text-xl font-semibold tracking-tight">Models</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            OpenRouter 公共目录；网关调用请使用模型{' '}
            <code className="rounded bg-muted px-1 py-0.5 text-xs">id</code>
            。与是否登录 ModelGate 无关。
          </p>
        </div>

        <Card className="border-border bg-muted/25 p-4">
          <div className="flex flex-wrap items-center gap-2">
            <div className="relative min-w-[200px] flex-1">
              <Search
                className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
                aria-hidden
              />
              <Input
                className="pl-9"
                placeholder="搜索模型…"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                aria-label="搜索模型"
              />
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled
              title="即将推出"
            >
              Compare
            </Button>
            <div
              className="flex items-center rounded-lg border border-border bg-background p-0.5"
              role="group"
              aria-label="视图"
            >
              <Button
                type="button"
                variant={view === 'list' ? 'secondary' : 'ghost'}
                size="icon-sm"
                aria-label="列表视图"
                onClick={() => setView('list')}
              >
                <LayoutList className="size-4" />
              </Button>
              <Button
                type="button"
                variant={view === 'grid' ? 'secondary' : 'ghost'}
                size="icon-sm"
                aria-label="网格视图"
                onClick={() => setView('grid')}
              >
                <LayoutGrid className="size-4" />
              </Button>
            </div>
            <label className="flex items-center gap-2 text-sm text-muted-foreground">
              排序
              <select
                className="h-9 rounded-md border border-input bg-background px-2 text-sm"
                value={sort}
                onChange={(e) =>
                  setSort(e.target.value as ExplorerSort)
                }
                aria-label="排序方式"
              >
                <option value="newest">Newest</option>
                <option value="name">Name</option>
              </select>
            </label>
            <div className="flex w-full flex-wrap items-center gap-2 sm:ml-auto sm:w-auto">
              {data != null && (
                <span className="text-sm text-muted-foreground">
                  共 {data.length} 个
                  {deferredQ.trim() || modality !== 'all' || series !== 'all'
                    ? `，当前 ${filtered.length} 个`
                    : ''}
                </span>
              )}
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={isPending || isFetching}
                onClick={() => void refetch()}
              >
                {isFetching ? '刷新中…' : '刷新'}
              </Button>
            </div>
          </div>
          <div className="mt-4">
            <ModelsExplorerModalityChips
              counts={counts}
              value={modality}
              onChange={setModality}
              size="md"
            />
          </div>
        </Card>

        {isPending && (
          <p className="text-sm text-muted-foreground">正在加载模型列表…</p>
        )}

        {isError && (
          <EmptyState
            title="加载失败"
            description={
              error instanceof Error ? error.message : '请稍后重试或检查网络。'
            }
          />
        )}

        {!isPending && !isError && filtered.length === 0 && (
          <EmptyState title="无匹配模型" description="试试调整筛选条件。" />
        )}

        {!isPending && !isError && view === 'list' && filtered.length > 0 && (
          <Card className="overflow-hidden py-0">
            <div
              ref={parentRef}
              className="overflow-auto"
              style={{ maxHeight: LIST_MAX_H }}
              role="list"
              aria-label="模型列表"
            >
              <div
                className="relative w-full"
                style={{ height: virtualizer.getTotalSize() }}
              >
                {virtualizer.getVirtualItems().map((vi) => {
                  const m = filtered[vi.index]
                  return (
                    <ModelsExplorerListRow
                      key={m.id}
                      model={m}
                      style={{
                        height: vi.size,
                        transform: `translateY(${vi.start}px)`,
                      }}
                      onCopy={() => void copyId(m.id)}
                    />
                  )
                })}
              </div>
            </div>
          </Card>
        )}

        {!isPending && !isError && view === 'grid' && filtered.length > 0 && (
          <div>
            <div
              className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3"
              role="list"
            >
              {filtered.slice(0, GRID_PREVIEW_CAP).map((m) => (
                <ModelsExplorerGridCard
                  key={m.id}
                  model={m}
                  onCopy={copyId}
                />
              ))}
            </div>
            {filtered.length > GRID_PREVIEW_CAP && (
              <p className="mt-3 text-center text-sm text-muted-foreground">
                网格视图仅展示前 {GRID_PREVIEW_CAP} 条，完整列表请切换到列表视图。
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
