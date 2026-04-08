import { useState } from 'react'
import { Link } from 'react-router-dom'
import { HTTPError } from 'ky'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/shared/empty-state'
import {
  useCreateByokProfile,
  useMyByokProfiles,
  useRevokeByokProfile,
} from '@/features/byok/hooks/use-byok-profiles'
import { useMyTeams } from '@/features/teams/hooks/use-teams'
import { useTeamStore } from '@/stores/team-store'

export function ByokProfilesPage() {
  const currentTeamId = useTeamStore((s) => s.currentTeamId)
  const { data: teamsRes } = useMyTeams()
  const teamName =
    currentTeamId == null
      ? null
      : teamsRes?.data?.find((t) => t.id === currentTeamId)?.name ?? null

  const { data, isLoading, isError, error, refetch } = useMyByokProfiles()
  const createMutation = useCreateByokProfile()
  const revokeMutation = useRevokeByokProfile()
  const [showCreateForm, setShowCreateForm] = useState(false)
  const [formName, setFormName] = useState('')
  const [formBaseUrl, setFormBaseUrl] = useState('https://api.openai.com/v1')
  const [formApiKey, setFormApiKey] = useState('')

  const is503 =
    isError && error instanceof HTTPError && error.response.status === 503

  const handleCreate = async () => {
    const base_url = formBaseUrl.trim()
    const api_key = formApiKey.trim()
    if (!base_url || !api_key) {
      window.alert('请填写上游 base_url 与 api_key')
      return
    }
    try {
      await createMutation.mutateAsync({
        ...(formName.trim() ? { name: formName.trim() } : {}),
        base_url,
        api_key,
      })
      setShowCreateForm(false)
      setFormName('')
      setFormApiKey('')
      setFormBaseUrl('https://api.openai.com/v1')
    } catch {
      /* ky */
    }
  }

  const handleRevoke = async (id: number, name: string) => {
    if (
      !window.confirm(
        `确定吊销 BYOK 配置「${name}」？\n吊销后无法再用于 X-MG-Byok-Id 转发。`,
      )
    ) {
      return
    }
    try {
      await revokeMutation.mutateAsync(id)
    } catch {
      /* */
    }
  }

  const rows = data?.data ?? []

  return (
    <section>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold">BYOK 上游密钥</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            自带 OpenAI 兼容上游的 <code className="text-xs">base_url</code> 与供应商{' '}
            <code className="text-xs">api_key</code>。调用{' '}
            <code className="text-xs">POST /v1/chat/completions</code> 时在请求头加入{' '}
            <code className="text-xs">X-MG-Byok-Id</code> 即可走该配置。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            disabled={is503}
            onClick={() => {
              setShowCreateForm((v) => !v)
            }}
          >
            {showCreateForm ? '取消' : '新建配置'}
          </Button>
        </div>
      </div>

      <p className="mt-4 rounded-md border border-border bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
        当前工作空间：
        {currentTeamId == null ? (
          <span className="text-foreground">个人空间</span>
        ) : (
          <span className="text-foreground">
            {teamName ?? `团队 #${currentTeamId}`}
          </span>
        )}
        。团队配置的新建 / 修改 / 吊销需要 owner 或 admin。
      </p>

      {is503 ? (
        <Card className="mt-6 border-amber-600/40 bg-amber-500/5 p-4">
          <p className="text-sm font-medium text-amber-900 dark:text-amber-200">
            服务器未启用 BYOK
          </p>
          <p className="mt-2 text-sm text-muted-foreground">
            请在部署配置中设置{' '}
            <code className="text-xs">[byok] master_key_hex</code>（64
            位十六进制）或环境变量 <code className="text-xs">BYOK_MASTER_KEY</code>
            ，并重启服务。
          </p>
        </Card>
      ) : null}

      {showCreateForm && !is503 ? (
        <Card className="mt-6 space-y-3 p-4">
          <p className="text-sm font-medium">新建 BYOK 配置</p>
          <label className="block text-sm">
            <span className="text-muted-foreground">名称（可选）</span>
            <Input
              className="mt-1"
              value={formName}
              onChange={(e) => setFormName(e.target.value)}
              placeholder="例如：生产 OpenAI"
            />
          </label>
          <label className="block text-sm">
            <span className="text-muted-foreground">上游 base_url</span>
            <Input
              className="mt-1 font-mono text-xs"
              value={formBaseUrl}
              onChange={(e) => setFormBaseUrl(e.target.value)}
              placeholder="https://api.openai.com/v1"
            />
          </label>
          <label className="block text-sm">
            <span className="text-muted-foreground">上游 API Key</span>
            <Input
              className="mt-1 font-mono text-xs"
              type="password"
              autoComplete="off"
              value={formApiKey}
              onChange={(e) => setFormApiKey(e.target.value)}
              placeholder="sk-…"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            密钥经服务器加密存储，不会完整回显。
          </p>
          <Button
            onClick={() => void handleCreate()}
            disabled={createMutation.isPending}
          >
            {createMutation.isPending ? '保存中…' : '保存'}
          </Button>
        </Card>
      ) : null}

      {createMutation.isError ? (
        <p className="mt-4 text-sm text-red-600 dark:text-red-400">
          创建失败（权限不足或校验错误）。团队上下文下需 owner/admin。
        </p>
      ) : null}

      <div className="mt-8">
        {isLoading ? (
          <p className="text-sm text-muted-foreground">加载中…</p>
        ) : isError && !is503 ? (
          <div className="space-y-2">
            <p className="text-sm text-red-600 dark:text-red-400">加载失败</p>
            <Button size="sm" variant="outline" onClick={() => void refetch()}>
              重试
            </Button>
          </div>
        ) : rows.length === 0 ? (
          <EmptyState
            title="暂无 BYOK 配置"
            description={
              is503
                ? '启用服务器 BYOK 主密钥后可在此管理。'
                : '点击「新建配置」添加第一个上游密钥。'
            }
          />
        ) : (
          <div className="overflow-x-auto rounded-lg border border-border">
            <table className="w-full min-w-[800px] text-left text-sm">
              <thead className="border-b border-border bg-muted/40">
                <tr>
                  <th className="px-4 py-3 font-medium">名称</th>
                  <th className="px-4 py-3 font-medium">base_url</th>
                  <th className="px-4 py-3 font-medium">密钥预览</th>
                  <th className="px-4 py-3 font-medium">状态</th>
                  <th className="px-4 py-3 font-medium text-right">操作</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={row.id} className="border-b border-border last:border-0">
                    <td className="px-4 py-3">
                      <Link
                        to={`/byok-profiles/${row.id}`}
                        className="font-medium text-primary hover:underline"
                      >
                        {row.name || `配置 #${row.id}`}
                      </Link>
                    </td>
                    <td className="max-w-[280px] truncate px-4 py-3 font-mono text-xs">
                      {row.base_url}
                    </td>
                    <td className="px-4 py-3 font-mono text-xs">
                      {row.api_key_preview}
                    </td>
                    <td className="px-4 py-3">
                      {row.revoked ? (
                        <span className="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                          已吊销
                        </span>
                      ) : (
                        <span className="rounded bg-emerald-600/15 px-2 py-0.5 text-xs text-emerald-800 dark:text-emerald-300">
                          有效
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <div className="flex flex-wrap justify-end gap-1">
                        <Button size="sm" variant="outline" asChild>
                          <Link to={`/byok-profiles/${row.id}`}>详情</Link>
                        </Button>
                        {!row.revoked ? (
                          <Button
                            size="sm"
                            variant="outline"
                            className="text-red-600 hover:bg-red-500/10 dark:text-red-400"
                            disabled={revokeMutation.isPending}
                            onClick={() =>
                              void handleRevoke(row.id, row.name || String(row.id))
                            }
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
