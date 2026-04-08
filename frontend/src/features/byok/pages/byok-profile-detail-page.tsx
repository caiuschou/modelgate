import { useState } from 'react'
import { HTTPError } from 'ky'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  useMyByokProfile,
  usePatchByokProfile,
  useRevokeByokProfile,
} from '@/features/byok/hooks/use-byok-profiles'
import type { ByokProfileSummary } from '@/features/byok/types'

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

function ByokProfileEditCard({ profile }: { profile: ByokProfileSummary }) {
  const patchMutation = usePatchByokProfile()
  const [name, setName] = useState(profile.name)
  const [baseUrl, setBaseUrl] = useState(profile.base_url)
  const [newApiKey, setNewApiKey] = useState('')

  const handleSave = async () => {
    const body: Record<string, unknown> = {}
    const n = name.trim()
    const bu = baseUrl.trim()
    if (n !== profile.name) {
      body.name = n || profile.name
    }
    if (bu !== profile.base_url) {
      if (!bu) {
        window.alert('base_url 不能为空')
        return
      }
      body.base_url = bu
    }
    const k = newApiKey.trim()
    if (k) {
      body.api_key = k
    }
    if (Object.keys(body).length === 0) {
      window.alert('没有变更')
      return
    }
    try {
      await patchMutation.mutateAsync({ id: profile.id, body })
    } catch {
      /* */
    }
  }

  return (
    <Card className="space-y-4 p-4">
      <h2 className="text-sm font-medium">编辑</h2>
      <label className="block text-sm">
        <span className="text-muted-foreground">名称</span>
        <Input
          className="mt-1"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
      </label>
      <label className="block text-sm">
        <span className="text-muted-foreground">base_url</span>
        <Input
          className="mt-1 font-mono text-xs"
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
        />
      </label>
      <label className="block text-sm">
        <span className="text-muted-foreground">轮换上游 API Key（留空则不修改）</span>
        <Input
          className="mt-1 font-mono text-xs"
          type="password"
          autoComplete="off"
          value={newApiKey}
          onChange={(e) => setNewApiKey(e.target.value)}
          placeholder="新的 sk-…"
        />
      </label>
      <Button
        onClick={() => void handleSave()}
        disabled={patchMutation.isPending}
      >
        {patchMutation.isPending ? '保存中…' : '保存更改'}
      </Button>
    </Card>
  )
}

export function ByokProfileDetailPage() {
  const { id: idParam } = useParams()
  const id = Number.parseInt(idParam ?? '', 10)
  const navigate = useNavigate()
  const { data, isLoading, isError, error } = useMyByokProfile(
    Number.isFinite(id) && id > 0 ? id : undefined,
  )
  const revokeMutation = useRevokeByokProfile()

  const is404 = isError && error instanceof HTTPError && error.response.status === 404
  const is503 = isError && error instanceof HTTPError && error.response.status === 503

  const handleRevoke = async () => {
    if (!data) {
      return
    }
    if (
      !window.confirm(
        `确定吊销「${data.name || `配置 #${data.id}`}」？此操作不可恢复。`,
      )
    ) {
      return
    }
    try {
      await revokeMutation.mutateAsync(id)
      navigate('/byok-profiles')
    } catch {
      /* */
    }
  }

  if (!Number.isFinite(id) || id <= 0) {
    return (
      <section>
        <p className="text-sm text-red-600">无效的配置 ID</p>
        <Button asChild className="mt-4" variant="outline">
          <Link to="/byok-profiles">返回列表</Link>
        </Button>
      </section>
    )
  }

  if (isLoading) {
    return <p className="text-sm text-muted-foreground">加载中…</p>
  }

  if (is503) {
    return (
      <section>
        <p className="text-sm text-amber-800 dark:text-amber-200">
          服务器未启用 BYOK（需配置 byok.master_key_hex）。
        </p>
        <Button asChild className="mt-4" variant="outline">
          <Link to="/byok-profiles">返回</Link>
        </Button>
      </section>
    )
  }

  if (is404 || isError || !data) {
    return (
      <section>
        <p className="text-sm text-red-600">未找到该配置或无权访问</p>
        <Button asChild className="mt-4" variant="outline">
          <Link to="/byok-profiles">返回列表</Link>
        </Button>
      </section>
    )
  }

  const revoked = data.revoked

  return (
    <section className="space-y-6">
      <div className="flex flex-wrap items-center gap-3">
        <Button variant="outline" size="sm" asChild>
          <Link to="/byok-profiles">← 返回</Link>
        </Button>
        <h1 className="text-2xl font-semibold">
          {data.name || `BYOK #${data.id}`}
        </h1>
        {revoked ? (
          <span className="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
            已吊销
          </span>
        ) : null}
      </div>

      <Card className="space-y-2 p-4 text-sm">
        <p>
          <span className="text-muted-foreground">配置 ID（用于网关头）</span>
        </p>
        <code className="block rounded border border-border bg-muted/40 px-3 py-2 font-mono text-xs">
          X-MG-Byok-Id: {data.id}
        </code>
        <p className="text-xs text-muted-foreground">
          使用与当前空间一致的网关 API Key 调用 Chat Completions 时带上上述请求头。
        </p>
      </Card>

      <Card className="space-y-4 p-4">
        <h2 className="text-sm font-medium">详情</h2>
        <dl className="grid gap-2 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-muted-foreground">密钥预览</dt>
            <dd className="font-mono text-xs">{data.api_key_preview}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">创建时间</dt>
            <dd>{formatTime(data.created_at)}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">更新时间</dt>
            <dd>{formatTime(data.updated_at)}</dd>
          </div>
        </dl>
      </Card>

      {!revoked ? (
        <ByokProfileEditCard key={`${data.id}-${data.updated_at}`} profile={data} />
      ) : null}

      {!revoked ? (
        <Card className="border-red-600/30 p-4">
          <h2 className="text-sm font-medium text-red-700 dark:text-red-400">危险操作</h2>
          <p className="mt-2 text-sm text-muted-foreground">
            吊销后该 ID 无法再用于转发。
          </p>
          <Button
            variant="outline"
            className="mt-4 text-red-600 hover:bg-red-500/10 dark:text-red-400"
            disabled={revokeMutation.isPending}
            onClick={() => void handleRevoke()}
          >
            吊销此配置
          </Button>
        </Card>
      ) : null}
    </section>
  )
}
