import { useState } from 'react'
import { HTTPError } from 'ky'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { apiPath, publicApi } from '@/lib/api-client'
import type { AdminDepositResponse } from '@/features/billing/types'

async function formatKyError(e: unknown): Promise<string> {
  if (e instanceof HTTPError) {
    try {
      const j = (await e.response.json()) as { error?: { message?: string } }
      return j.error?.message ?? e.message
    } catch {
      return e.message
    }
  }
  return e instanceof Error ? e.message : String(e)
}

export function AdminRechargePage() {
  const [username, setUsername] = useState('')
  const [amountInput, setAmountInput] = useState('10')
  const [password, setPassword] = useState('')
  const [pending, setPending] = useState(false)
  const [result, setResult] = useState<AdminDepositResponse | null>(null)
  const [error, setError] = useState<string | null>(null)

  const submit = async () => {
    setError(null)
    setResult(null)
    const amount = Number.parseFloat(amountInput.replace(',', '.'))
    if (!username.trim()) {
      setError('请输入控制台用户名')
      return
    }
    if (!Number.isFinite(amount) || amount <= 0) {
      setError('请输入有效的 USD 金额')
      return
    }
    if (!password) {
      setError('请输入管理密码')
      return
    }
    setPending(true)
    try {
      const res = await publicApi
        .post(apiPath('api/v1/billing/admin-deposit'), {
          json: { username: username.trim(), amount_usd: amount },
          headers: { Authorization: `Bearer ${password}` },
        })
        .json<AdminDepositResponse>()
      setResult(res)
    } catch (e: unknown) {
      setError(await formatKyError(e))
    } finally {
      setPending(false)
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>管理充值</CardTitle>
          <p className="text-muted-foreground text-sm">
            为指定控制台用户增加余额（USD）。需与服务端 <code className="text-xs">billing.admin_deposit_password</code>{' '}
            一致。
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-1">
            <label className="text-sm font-medium" htmlFor="admin-recharge-username">
              控制台用户名
            </label>
            <Input
              id="admin-recharge-username"
              autoComplete="username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="与注册用户名一致"
            />
          </div>
          <div className="space-y-1">
            <label className="text-sm font-medium" htmlFor="admin-recharge-amount">
              充值金额（USD）
            </label>
            <Input
              id="admin-recharge-amount"
              inputMode="decimal"
              value={amountInput}
              onChange={(e) => setAmountInput(e.target.value)}
              placeholder="例如 10"
            />
          </div>
          <div className="space-y-1">
            <label className="text-sm font-medium" htmlFor="admin-recharge-password">
              管理密码
            </label>
            <Input
              id="admin-recharge-password"
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          </div>
          {error ? (
            <p className="text-destructive text-sm" role="alert">
              {error}
            </p>
          ) : null}
          {result ? (
            <p className="text-sm text-muted-foreground" role="status">
              用户 <span className="font-medium text-foreground">{result.username}</span>（id {result.user_id}
              ）入账后余额 <span className="font-mono tabular-nums">{result.balance_usd}</span> USD。
            </p>
          ) : null}
          <Button type="button" className="w-full" disabled={pending} onClick={() => void submit()}>
            {pending ? '处理中…' : '确认充值'}
          </Button>
        </CardContent>
      </Card>
    </main>
  )
}
