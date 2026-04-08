import { useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  useMyApiKey,
  usePatchMyApiKey,
  useRevokeMyApiKey,
} from '@/features/api-keys/hooks/use-api-keys'
import type { ApiKeySummary } from '@/features/api-keys/types'
import { useMyByokProfiles } from '@/features/byok/hooks/use-byok-profiles'

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

function toDatetimeLocal(ts: number): string {
  const d = new Date(ts * 1000)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}

function fromDatetimeLocal(s: string): number | null {
  const t = Date.parse(s)
  return Number.isFinite(t) ? Math.floor(t / 1000) : null
}

/** Remount policy editor when server-backed fields change (avoids setState in an effect). */
function policiesEditorKey(d: ApiKeySummary): string {
  return [
    d.id,
    d.expires_at ?? '',
    d.quota_monthly_tokens ?? '',
    (d.model_allowlist ?? []).join('\0'),
    (d.ip_allowlist ?? []).join('\0'),
    d.default_byok_profile_id ?? '',
  ].join('|')
}

function ApiKeyPoliciesEditor({
  data,
  patchMutation,
}: {
  data: ApiKeySummary
  patchMutation: ReturnType<typeof usePatchMyApiKey>
}) {
  const [expiresInput, setExpiresInput] = useState(
    () => (data.expires_at ? toDatetimeLocal(data.expires_at) : ''),
  )
  const [quotaInput, setQuotaInput] = useState(() =>
    data.quota_monthly_tokens != null ? String(data.quota_monthly_tokens) : '',
  )
  const [modelsText, setModelsText] = useState(
    () => data.model_allowlist?.join(', ') ?? '',
  )
  const [ipsText, setIpsText] = useState(
    () => data.ip_allowlist?.join(', ') ?? '',
  )

  const handleSavePolicies = async () => {
    const models = modelsText
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean)
    const ips = ipsText
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean)
    const quotaParsed =
      quotaInput.trim() === '' ? null : Number.parseInt(quotaInput, 10)
    if (quotaInput.trim() !== '' && Number.isNaN(quotaParsed as number)) {
      window.alert('月度配额必须是数字')
      return
    }
    let expiresAt: number | null | undefined
    if (expiresInput.trim() === '') {
      expiresAt = null
    } else {
      const ts = fromDatetimeLocal(expiresInput)
      if (ts == null) {
        window.alert('过期时间格式无效')
        return
      }
      expiresAt = ts
    }
    try {
      await patchMutation.mutateAsync({
        id: data.id,
        body: {
          expires_at: expiresAt,
          quota_monthly_tokens: quotaParsed,
          model_allowlist: models.length > 0 ? models : null,
          ip_allowlist: ips.length > 0 ? ips : null,
        },
      })
    } catch {
      /* */
    }
  }

  return (
    <Card className="space-y-4 p-4">
      <h2 className="text-sm font-medium">编辑策略</h2>
      <p className="text-xs text-muted-foreground">
        保存后将更新过期时间、月度配额与模型 / IP 白名单（留空表示清除限制）。
      </p>
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="space-y-1.5 sm:col-span-2">
          <label htmlFor="expires" className="text-sm font-medium">
            过期时间（本地）
          </label>
          <Input
            id="expires"
            type="datetime-local"
            value={expiresInput}
            onChange={(e) => setExpiresInput(e.target.value)}
          />
        </div>
        <div className="space-y-1.5">
          <label htmlFor="quota" className="text-sm font-medium">
            月度 Token 配额
          </label>
          <Input
            id="quota"
            type="number"
            min={0}
            placeholder="不限制请留空"
            value={quotaInput}
            onChange={(e) => setQuotaInput(e.target.value)}
          />
        </div>
        <div className="space-y-1.5">
          <label htmlFor="models" className="text-sm font-medium">
            模型白名单
          </label>
          <Input
            id="models"
            placeholder="逗号分隔，如 gpt-4, gpt-3.5-turbo"
            value={modelsText}
            onChange={(e) => setModelsText(e.target.value)}
          />
        </div>
        <div className="space-y-1.5 sm:col-span-2">
          <label htmlFor="ips" className="text-sm font-medium">
            IP 白名单（CIDR 或单 IP）
          </label>
          <Input
            id="ips"
            placeholder="逗号分隔"
            value={ipsText}
            onChange={(e) => setIpsText(e.target.value)}
          />
        </div>
      </div>
      <Button
        size="sm"
        disabled={patchMutation.isPending}
        onClick={() => void handleSavePolicies()}
      >
        保存策略
      </Button>
    </Card>
  )
}

function ApiKeyDefaultUpstreamSection({
  data,
  patchMutation,
}: {
  data: ApiKeySummary
  patchMutation: ReturnType<typeof usePatchMyApiKey>
}) {
  const { data: byokRes, isLoading, isError } = useMyByokProfiles()
  const currentPid = data.default_byok_profile_id
  const initial =
    currentPid != null ? String(currentPid) : 'platform'
  const [selection, setSelection] = useState(initial)

  const activeProfiles = (byokRes?.data ?? []).filter((p) => !p.revoked)
  const bound =
    currentPid != null
      ? (byokRes?.data ?? []).find((p) => p.id === currentPid)
      : undefined
  const stuckId =
    currentPid != null && !activeProfiles.some((p) => p.id === currentPid)
      ? currentPid
      : null

  const defaultLabel =
    currentPid == null
      ? 'ModelGate（实例上游）'
      : bound
        ? bound.revoked
          ? `BYOK「${bound.name}」（已吊销，请改选或清除）`
          : `BYOK「${bound.name}」`
        : `BYOK #${currentPid}（当前列表不可用）`

  const handleSave = async () => {
    if (selection === 'platform') {
      try {
        await patchMutation.mutateAsync({
          id: data.id,
          body: { default_byok_profile_id: null },
        })
      } catch {
        /* */
      }
      return
    }
    const pid = Number.parseInt(selection, 10)
    if (Number.isNaN(pid)) {
      window.alert('请选择有效的 BYOK 配置')
      return
    }
    try {
      await patchMutation.mutateAsync({
        id: data.id,
        body: { default_byok_profile_id: pid },
      })
    } catch {
      /* */
    }
  }

  return (
    <Card className="space-y-4 p-4">
      <h2 className="text-sm font-medium">默认上游（Chat）</h2>
      <p className="text-xs text-muted-foreground">
        未携带 <code className="text-xs">X-MG-Byok-Id</code> 时使用的上游。可选 ModelGate
        配置的 <code className="text-xs">[upstream]</code>，或当前空间下某一 BYOK。请求头{' '}
        <code className="text-xs">X-MG-Use-Platform-Upstream: 1</code> 可强制走平台上游。
      </p>
      <div>
        <p className="text-xs text-muted-foreground">当前已保存</p>
        <p className="text-sm">{defaultLabel}</p>
      </div>
      <label className="block space-y-1.5">
        <span className="text-sm font-medium">改选默认</span>
        <select
          className="w-full rounded border border-border bg-background px-2 py-2 text-sm"
          value={selection}
          onChange={(e) => setSelection(e.target.value)}
          aria-label="默认 Chat 上游"
          disabled={isLoading}
        >
          <option value="platform">ModelGate（[upstream]）</option>
          {activeProfiles.map((p) => (
            <option key={p.id} value={String(p.id)}>
              {p.name} (#{p.id})
            </option>
          ))}
          {stuckId != null ? (
            <option value={String(stuckId)}>
              已绑定 #{stuckId}（不可用，请尽快改选）
            </option>
          ) : null}
        </select>
      </label>
      {isError ? (
        <p className="text-xs text-amber-700 dark:text-amber-300">
          无法加载 BYOK 列表（例如未配置服务端主密钥）。仍可保存为 ModelGate；要绑定 BYOK
          请先在本空间完成 BYOK 配置。
        </p>
      ) : null}
      <Button
        size="sm"
        disabled={patchMutation.isPending || isLoading}
        onClick={() => void handleSave()}
      >
        保存默认上游
      </Button>
    </Card>
  )
}

export function ApiKeyDetailPage() {
  const { id } = useParams<{ id: string }>()
  const keyId = Number(id)
  const { data, isLoading, isError, refetch } = useMyApiKey(
    Number.isFinite(keyId) ? keyId : undefined,
  )
  const patchMutation = usePatchMyApiKey()
  const revokeMutation = useRevokeMyApiKey()
  const [rotateOpen, setRotateOpen] = useState(false)

  if (!Number.isFinite(keyId) || keyId <= 0) {
    return (
      <section>
        <p className="text-sm text-muted-foreground">无效的密钥 ID</p>
        <Link to="/api-keys" className="mt-2 inline-block text-primary underline">
          返回列表
        </Link>
      </section>
    )
  }

  if (isLoading) {
    return <p className="text-sm text-muted-foreground">加载中…</p>
  }

  if (isError || !data) {
    return (
      <section className="space-y-2">
        <p className="text-sm text-red-600">加载失败</p>
        <Button size="sm" variant="outline" onClick={() => void refetch()}>
          重试
        </Button>
        <Link to="/api-keys" className="ml-2 text-primary underline">
          返回列表
        </Link>
      </section>
    )
  }

  const handleRevoke = async () => {
    if (
      !window.confirm(
        `确定吊销「${data.name}」？吊销后不可恢复，请先完成轮换。`,
      )
    ) {
      return
    }
    try {
      await revokeMutation.mutateAsync(data.id)
      window.location.href = '/api-keys'
    } catch {
      /* */
    }
  }

  const handleToggleDisable = async () => {
    const next = !data.disabled
    if (
      !window.confirm(
        next
          ? '确定禁用该密钥？'
          : '确定启用该密钥？',
      )
    ) {
      return
    }
    try {
      await patchMutation.mutateAsync({
        id: data.id,
        body: { disabled: next },
      })
    } catch {
      /* */
    }
  }

  return (
    <section className="space-y-6">
      <div className="text-sm text-muted-foreground">
        <Link to="/api-keys" className="text-primary hover:underline">
          API 密钥
        </Link>
        <span className="mx-2">/</span>
        <span>{data.name}</span>
      </div>

      <div>
        <h1 className="text-2xl font-semibold">{data.name}</h1>
        <p className="mt-1 font-mono text-xs text-muted-foreground">{data.preview}</p>
      </div>

      <Card className="space-y-4 p-4">
        <h2 className="text-sm font-medium">元数据</h2>
        <dl className="grid gap-2 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-muted-foreground">描述</dt>
            <dd>{data.description || '—'}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">状态</dt>
            <dd>{data.status}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">创建时间</dt>
            <dd>{formatTime(data.created_at)}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">最后使用</dt>
            <dd>
              {data.last_used_at ? formatTime(data.last_used_at) : '从未使用'}
            </dd>
          </div>
          {data.expires_at ? (
            <div>
              <dt className="text-muted-foreground">过期时间</dt>
              <dd>{formatTime(data.expires_at)}</dd>
            </div>
          ) : null}
          {data.quota_monthly_tokens != null ? (
            <div>
              <dt className="text-muted-foreground">月度 Token 配额</dt>
              <dd>
                已用 {data.quota_used_tokens} / {data.quota_monthly_tokens}
              </dd>
            </div>
          ) : null}
        </dl>
      </Card>

      {!data.revoked && data.status !== 'expired' ? (
        <>
          <ApiKeyDefaultUpstreamSection
            key={`def-up-${data.id}-${data.default_byok_profile_id ?? 'platform'}`}
            data={data}
            patchMutation={patchMutation}
          />
          <ApiKeyPoliciesEditor
            key={policiesEditorKey(data)}
            data={data}
            patchMutation={patchMutation}
          />
        </>
      ) : null}

      <div className="flex flex-wrap gap-2">
        <Button variant="outline" asChild>
          <Link to={`/logs?token_id=${data.id}`}>相关日志</Link>
        </Button>
        {!data.revoked && data.status !== 'expired' ? (
          <>
            <Button
              variant="outline"
              disabled={patchMutation.isPending}
              onClick={() => void handleToggleDisable()}
            >
              {data.disabled ? '启用' : '禁用'}
            </Button>
            <Button
              variant="outline"
              onClick={() => setRotateOpen((v) => !v)}
            >
              轮换指引
            </Button>
            <Button
              variant="outline"
              className="text-red-600 hover:bg-red-500/10"
              disabled={revokeMutation.isPending}
              onClick={() => void handleRevoke()}
            >
              吊销
            </Button>
          </>
        ) : null}
      </div>

      {rotateOpen ? (
        <Card className="space-y-3 border-amber-600/40 bg-amber-500/5 p-4">
          <h2 className="text-sm font-medium text-amber-900 dark:text-amber-200">
            安全轮换（人工确认）
          </h2>
          <ol className="list-decimal space-y-2 pl-5 text-sm">
            <li>
              在列表页点击「新建密钥」，生成新密钥并保存到环境变量（如{' '}
              <code className="text-xs">MODELGATE_API_KEY</code>）。
            </li>
            <li>更新应用配置，勿将密钥提交到版本库。</li>
            <li>发起一次测试请求，或在日志中心确认新密钥产生的记录。</li>
            <li>确认无旧密钥流量后，回到列表吊销本密钥。</li>
          </ol>
          <Button size="sm" asChild>
            <Link to="/api-keys">去列表新建密钥</Link>
          </Button>
        </Card>
      ) : null}
    </section>
  )
}
