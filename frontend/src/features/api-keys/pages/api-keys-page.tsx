import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { EmptyState } from '@/components/shared/empty-state'
import {
  useCreateMyApiKey,
  useMyApiKeys,
  useRevokeMyApiKey,
} from '@/features/api-keys/hooks/use-api-keys'

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

export function ApiKeysPage() {
  const { data, isLoading, isError, refetch } = useMyApiKeys()
  const createMutation = useCreateMyApiKey()
  const revokeMutation = useRevokeMyApiKey()
  const [newKeySecret, setNewKeySecret] = useState<string | null>(null)
  const [copyHint, setCopyHint] = useState<string | null>(null)

  const handleCreate = async () => {
    setNewKeySecret(null)
    setCopyHint(null)
    try {
      const res = await createMutation.mutateAsync()
      setNewKeySecret(res.api_key)
    } catch {
      /* ky throws; global handling optional */
    }
  }

  const handleCopy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopyHint('已复制到剪贴板')
      setTimeout(() => setCopyHint(null), 2000)
    } catch {
      setCopyHint('复制失败，请手动选择复制')
    }
  }

  const handleRevoke = async (id: number, preview: string) => {
    if (
      !window.confirm(
        `确定吊销该 API 密钥？\n${preview}\n吊销后立即失效，且不可恢复。`,
      )
    ) {
      return
    }
    try {
      await revokeMutation.mutateAsync(id)
    } catch {
      /* 401 等由 apiClient 处理 */
    }
  }

  const rows = data?.data ?? []

  return (
    <section>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold">API 密钥</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            用于调用网关（<code className="text-xs">Authorization: Bearer</code>
            ）。与模型用量中的 Token 计数不同。
          </p>
        </div>
        <Button
          onClick={() => void handleCreate()}
          disabled={createMutation.isPending}
        >
          {createMutation.isPending ? '创建中…' : '新建密钥'}
        </Button>
      </div>

      {newKeySecret ? (
        <Card className="mt-6 border-amber-600/40 bg-amber-500/5 p-4">
          <p className="text-sm font-medium text-amber-900 dark:text-amber-200">
            请立即保存 — 完整密钥仅显示这一次
          </p>
          <pre className="mt-2 overflow-x-auto rounded border border-border bg-muted/50 p-3 font-mono text-xs break-all">
            {newKeySecret}
          </pre>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button size="sm" variant="outline" onClick={() => void handleCopy(newKeySecret)}>
              复制完整密钥
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setNewKeySecret(null)}>
              我已保存
            </Button>
          </div>
          {copyHint ? (
            <p className="mt-2 text-xs text-muted-foreground">{copyHint}</p>
          ) : null}
        </Card>
      ) : null}

      {createMutation.isError ? (
        <p className="mt-4 text-sm text-red-600 dark:text-red-400">
          创建失败，请稍后重试或检查是否已登录。
        </p>
      ) : null}

      <div className="mt-8">
        {isLoading ? (
          <p className="text-sm text-muted-foreground">加载中…</p>
        ) : isError ? (
          <div className="space-y-2">
            <p className="text-sm text-red-600 dark:text-red-400">加载失败</p>
            <Button size="sm" variant="outline" onClick={() => void refetch()}>
              重试
            </Button>
          </div>
        ) : rows.length === 0 ? (
          <EmptyState
            title="暂无密钥"
            description="点击「新建密钥」生成第一个 API 密钥。"
          />
        ) : (
          <div className="overflow-x-auto rounded-lg border border-border">
            <table className="w-full min-w-[640px] text-left text-sm">
              <thead className="border-b border-border bg-muted/40">
                <tr>
                  <th className="px-4 py-3 font-medium">预览</th>
                  <th className="px-4 py-3 font-medium">创建时间</th>
                  <th className="px-4 py-3 font-medium">状态</th>
                  <th className="px-4 py-3 font-medium text-right">操作</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={row.id} className="border-b border-border last:border-0">
                    <td className="px-4 py-3 font-mono text-xs">{row.preview}</td>
                    <td className="px-4 py-3 text-muted-foreground">
                      {formatTime(row.created_at)}
                    </td>
                    <td className="px-4 py-3">
                      {row.revoked ? (
                        <span className="rounded bg-muted px-2 py-0.5 text-xs">已吊销</span>
                      ) : (
                        <span className="rounded bg-emerald-600/15 px-2 py-0.5 text-xs text-emerald-800 dark:text-emerald-300">
                          有效
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-right">
                      {!row.revoked ? (
                        <Button
                          size="sm"
                          variant="outline"
                          className="text-red-600 hover:bg-red-500/10 dark:text-red-400"
                          disabled={revokeMutation.isPending}
                          onClick={() => void handleRevoke(row.id, row.preview)}
                        >
                          吊销
                        </Button>
                      ) : (
                        <span className="text-xs text-muted-foreground">—</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </section>
  )
}
