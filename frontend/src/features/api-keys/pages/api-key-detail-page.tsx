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
import type { ApiKeySummary, UpstreamPoolEntry } from '@/features/api-keys/types'
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

const USD_MINOR_EXP = 15

/** Display-only; sufficient for typical monthly USD caps. */
function minorStrToApproxUsd(s: string): string {
  try {
    const x = Number(BigInt(s.trim())) / 10 ** USD_MINOR_EXP
    if (!Number.isFinite(x)) return s
    return String(x)
  } catch {
    return s
  }
}

function usdInputToMinorStr(raw: string): string | null {
  const t = raw.trim()
  if (t === '') return null
  const x = Number(t)
  if (!Number.isFinite(x) || x <= 0) return null
  try {
    return BigInt(Math.round(x * 10 ** USD_MINOR_EXP)).toString()
  } catch {
    return null
  }
}

/** Remount policy editor when server-backed fields change (avoids setState in an effect). */
function policiesEditorKey(d: ApiKeySummary): string {
  return [
    d.id,
    d.expires_at ?? '',
    d.quota_monthly_tokens ?? '',
    d.max_concurrent_requests ?? '',
    d.quota_monthly_spend_minor ?? '',
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
  const [maxConcurrentInput, setMaxConcurrentInput] = useState(() =>
    data.max_concurrent_requests != null
      ? String(data.max_concurrent_requests)
      : '',
  )
  const [spendUsdInput, setSpendUsdInput] = useState(() =>
    data.quota_monthly_spend_minor != null && data.quota_monthly_spend_minor !== ''
      ? minorStrToApproxUsd(data.quota_monthly_spend_minor)
      : '',
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
    let maxConcurrent: number | null = null
    if (maxConcurrentInput.trim() !== '') {
      const n = Number.parseInt(maxConcurrentInput, 10)
      if (Number.isNaN(n) || n < 0 || n > 65_535) {
        window.alert('并发上限须为 0–65535 的整数（留空表示不限制）')
        return
      }
      maxConcurrent = n === 0 ? null : n
    }
    let spendMinor: string | null = null
    if (spendUsdInput.trim() !== '') {
      const m = usdInputToMinorStr(spendUsdInput)
      if (m == null) {
        window.alert('月度消费上限（USD）须为正数')
        return
      }
      spendMinor = m
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
          max_concurrent_requests: maxConcurrent,
          quota_monthly_spend_minor: spendMinor,
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
        保存后将更新过期时间、月度配额、并发与消费上限及模型 / IP 白名单（留空表示清除对应限制）。
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
          <label htmlFor="max-concurrent" className="text-sm font-medium">
            最大并发请求数
          </label>
          <Input
            id="max-concurrent"
            type="number"
            min={0}
            max={65535}
            placeholder="不限制请留空；0 同不限制"
            value={maxConcurrentInput}
            onChange={(e) => setMaxConcurrentInput(e.target.value)}
          />
        </div>
        <div className="space-y-1.5">
          <label htmlFor="spend-cap-usd" className="text-sm font-medium">
            月度消费上限（USD）
          </label>
          <Input
            id="spend-cap-usd"
            type="text"
            inputMode="decimal"
            placeholder="不限制请留空；计费开启时按自然月累计"
            value={spendUsdInput}
            onChange={(e) => setSpendUsdInput(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            与账户余额一致使用 USD minor（scale {USD_MINOR_EXP}）；仅非 BYOK 且平台计费成功扣款时累计。
          </p>
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

function poolRowsFromServer(data: ApiKeySummary): UpstreamPoolEntry[] {
  const p = data.upstream_pool
  if (p != null && p.length > 0) {
    return p
  }
  return [{ kind: 'platform' }]
}

function ApiKeySessionAffinitySection({
  data,
  patchMutation,
}: {
  data: ApiKeySummary
  patchMutation: ReturnType<typeof usePatchMyApiKey>
}) {
  const { data: byokRes, isLoading, isError } = useMyByokProfiles()
  const activeProfiles = (byokRes?.data ?? []).filter((p) => !p.revoked)
  const [enabled, setEnabled] = useState(
    () => data.session_affinity_enabled ?? false,
  )
  const [rows, setRows] = useState<UpstreamPoolEntry[]>(() =>
    poolRowsFromServer(data),
  )

  const handleSave = async () => {
    if (enabled && rows.length === 0) {
      window.alert('启用会话亲和时，上游池至少需一项')
      return
    }
    let platformRows = 0
    const seenByok = new Set<number>()
    for (const r of rows) {
      if (r.kind === 'platform') {
        platformRows += 1
      } else {
        if (seenByok.has(r.byok_profile_id)) {
          window.alert('上游池中 BYOK 不可重复')
          return
        }
        seenByok.add(r.byok_profile_id)
      }
    }
    if (platformRows > 1) {
      window.alert('上游池最多包含一条平台（ModelGate）线路')
      return
    }
    try {
      await patchMutation.mutateAsync({
        id: data.id,
        body: {
          session_affinity_enabled: enabled,
          upstream_pool: rows,
        },
      })
    } catch {
      /* ky throws */
    }
  }

  const setRowKind = (index: number, raw: string) => {
    setRows((prev) => {
      const next = [...prev]
      if (raw === 'platform') {
        next[index] = { kind: 'platform' }
      } else {
        const pid = Number.parseInt(raw, 10)
        if (!Number.isNaN(pid)) {
          next[index] = { kind: 'byok', byok_profile_id: pid }
        }
      }
      return next
    })
  }

  const rowValue = (r: UpstreamPoolEntry): string => {
    if (r.kind === 'platform') return 'platform'
    return String(r.byok_profile_id)
  }

  return (
    <Card className="space-y-4 p-4">
      <h2 className="text-sm font-medium">会话上游（亲和）</h2>
      <p className="text-xs text-muted-foreground">
        开启后，同一 <code className="text-xs">X-Thread-Id</code>（或请求体{' '}
        <code className="text-xs">user</code>）的会话固定到一条上游；新会话按下列顺序轮流分配（Round
        Robin）并写入绑定。未传会话键时仍走上方「默认 Chat 上游」。
      </p>
      <label className="flex cursor-pointer items-center gap-2 text-sm">
        <input
          type="checkbox"
          className="rounded border border-border"
          checked={enabled}
          onChange={(e) => setEnabled(e.target.checked)}
          aria-label="启用会话亲和"
        />
        启用会话亲和与上游池
      </label>
      <div className="space-y-2">
        <p className="text-xs text-muted-foreground">上游池顺序（由上到下即 RR 顺序）</p>
        {rows.map((r, i) => (
          <div key={`row-${i}`} className="flex flex-wrap items-center gap-2">
            <span className="text-xs text-muted-foreground">#{i + 1}</span>
            <select
              className="min-w-[200px] flex-1 rounded border border-border bg-background px-2 py-2 text-sm"
              value={rowValue(r)}
              onChange={(e) => setRowKind(i, e.target.value)}
              disabled={isLoading}
              aria-label={`上游池第 ${i + 1} 项`}
            >
              <option value="platform">ModelGate（[upstream]）</option>
              {activeProfiles.map((p) => (
                <option key={p.id} value={String(p.id)}>
                  BYOK「{p.name}」(#{p.id})
                </option>
              ))}
            </select>
            <Button
              type="button"
              size="sm"
              variant="outline"
              disabled={rows.length <= 1}
              onClick={() => setRows((prev) => prev.filter((_, j) => j !== i))}
            >
              移除
            </Button>
          </div>
        ))}
        <Button
          type="button"
          size="sm"
          variant="secondary"
          onClick={() => {
            setRows((prev) => {
              const hasPlat = prev.some((x) => x.kind === 'platform')
              if (!hasPlat) {
                return [...prev, { kind: 'platform' }]
              }
              const fp = activeProfiles[0]?.id
              if (fp == null) {
                window.alert(
                  '已有一条平台线路；请先在当前空间创建 BYOK 后再添加第二项。',
                )
                return prev
              }
              return [...prev, { kind: 'byok', byok_profile_id: fp }]
            })
          }}
        >
          添加上游
        </Button>
      </div>
      {isError ? (
        <p className="text-xs text-amber-700 dark:text-amber-300">
          无法加载 BYOK 列表；仍可配置平台线路。
        </p>
      ) : null}
      <Button
        size="sm"
        disabled={patchMutation.isPending || isLoading}
        onClick={() => void handleSave()}
      >
        保存会话上游
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
          {data.max_concurrent_requests != null ? (
            <div>
              <dt className="text-muted-foreground">最大并发</dt>
              <dd>{data.max_concurrent_requests}</dd>
            </div>
          ) : null}
          {data.quota_monthly_spend_minor != null &&
          data.quota_monthly_spend_minor !== '' ? (
            <div>
              <dt className="text-muted-foreground">月度消费上限（USD 约）</dt>
              <dd className="font-mono text-xs">
                已用{' '}
                {minorStrToApproxUsd(data.quota_used_spend_minor ?? '0')} /{' '}
                {minorStrToApproxUsd(data.quota_monthly_spend_minor)}
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
          <ApiKeySessionAffinitySection
            key={`sess-${data.id}-${data.session_affinity_enabled ? '1' : '0'}-${JSON.stringify(data.upstream_pool ?? [])}`}
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
