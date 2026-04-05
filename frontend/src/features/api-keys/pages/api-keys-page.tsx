import { useState } from 'react'
import { Link } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/shared/empty-state'
import {
  useCreateMyApiKey,
  useMyApiKeys,
  usePatchMyApiKey,
  useRevokeMyApiKey,
} from '@/features/api-keys/hooks/use-api-keys'

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

function statusLabel(status: string): string {
  switch (status) {
    case 'active':
      return '有效'
    case 'disabled':
      return '已禁用'
    case 'revoked':
      return '已吊销'
    case 'expired':
      return '已过期'
    default:
      return status
  }
}

export function ApiKeysPage() {
  const { data, isLoading, isError, refetch } = useMyApiKeys()
  const createMutation = useCreateMyApiKey()
  const revokeMutation = useRevokeMyApiKey()
  const patchMutation = usePatchMyApiKey()
  const [newKeySecret, setNewKeySecret] = useState<string | null>(null)
  const [copyHint, setCopyHint] = useState<string | null>(null)
  const [showCreateForm, setShowCreateForm] = useState(false)
  const [formName, setFormName] = useState('')
  const [formDesc, setFormDesc] = useState('')

  const handleCreate = async () => {
    setNewKeySecret(null)
    setCopyHint(null)
    const name = formName.trim() || '未命名密钥'
    try {
      const res = await createMutation.mutateAsync({
        name,
        ...(formDesc.trim() ? { description: formDesc.trim() } : {}),
      })
      setNewKeySecret(res.api_key)
      setShowCreateForm(false)
      setFormName('')
      setFormDesc('')
    } catch {
      /* ky throws */
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

  const handleToggleDisable = async (
    id: number,
    disabled: boolean,
    name: string,
  ) => {
    const next = !disabled
    if (
      !window.confirm(
        next
          ? `确定禁用「${name}」？禁用后该密钥无法调用网关，可随时重新启用。`
          : `确定启用「${name}」？`,
      )
    ) {
      return
    }
    try {
      await patchMutation.mutateAsync({
        id,
        body: { disabled: next },
      })
    } catch {
      /* handled globally */
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
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            onClick={() => {
              setShowCreateForm((v) => !v)
              setNewKeySecret(null)
            }}
          >
            {showCreateForm ? '取消' : '新建密钥'}
          </Button>
        </div>
      </div>

      {showCreateForm ? (
        <Card className="mt-6 space-y-3 p-4">
          <p className="text-sm font-medium">新建 API 密钥</p>
          <label className="block text-sm">
            <span className="text-muted-foreground">名称（必填）</span>
            <Input
              className="mt-1"
              value={formName}
              onChange={(e) => setFormName(e.target.value)}
              placeholder="例如：生产-支付助手"
            />
          </label>
          <label className="block text-sm">
            <span className="text-muted-foreground">描述（可选）</span>
            <Input
              className="mt-1"
              value={formDesc}
              onChange={(e) => setFormDesc(e.target.value)}
              placeholder="备注用途或环境"
            />
          </label>
          <Button
            onClick={() => void handleCreate()}
            disabled={createMutation.isPending}
          >
            {createMutation.isPending ? '创建中…' : '生成密钥'}
          </Button>
        </Card>
      ) : null}

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
            <table className="w-full min-w-[880px] text-left text-sm">
              <thead className="border-b border-border bg-muted/40">
                <tr>
                  <th className="px-4 py-3 font-medium">名称</th>
                  <th className="px-4 py-3 font-medium">预览</th>
                  <th className="px-4 py-3 font-medium">最后使用</th>
                  <th className="px-4 py-3 font-medium">状态</th>
                  <th className="px-4 py-3 font-medium text-right">操作</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={row.id} className="border-b border-border last:border-0">
                    <td className="px-4 py-3">
                      <Link
                        to={`/api-keys/${row.id}`}
                        className="font-medium text-primary hover:underline"
                      >
                        {row.name}
                      </Link>
                    </td>
                    <td className="px-4 py-3 font-mono text-xs">{row.preview}</td>
                    <td className="px-4 py-3 text-muted-foreground">
                      {row.last_used_at
                        ? formatTime(row.last_used_at)
                        : '从未使用'}
                    </td>
                    <td className="px-4 py-3">
                      <span
                        className={`rounded px-2 py-0.5 text-xs ${
                          row.status === 'active'
                            ? 'bg-emerald-600/15 text-emerald-800 dark:text-emerald-300'
                            : row.status === 'disabled'
                              ? 'bg-amber-600/15 text-amber-800 dark:text-amber-300'
                              : 'bg-muted text-muted-foreground'
                        }`}
                      >
                        {statusLabel(row.status)}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-right">
                      <div className="flex flex-wrap justify-end gap-1">
                        <Button size="sm" variant="outline" asChild>
                          <Link
                            to={`/logs?token_id=${encodeURIComponent(String(row.id))}`}
                          >
                            相关日志
                          </Link>
                        </Button>
                        <Button size="sm" variant="outline" asChild>
                          <Link to={`/api-keys/${row.id}`}>详情</Link>
                        </Button>
                        {!row.revoked && row.status !== 'expired' ? (
                          <Button
                            size="sm"
                            variant="outline"
                            disabled={patchMutation.isPending}
                            onClick={() =>
                              void handleToggleDisable(
                                row.id,
                                row.disabled,
                                row.name,
                              )
                            }
                          >
                            {row.disabled ? '启用' : '禁用'}
                          </Button>
                        ) : null}
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
                      </div>
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
