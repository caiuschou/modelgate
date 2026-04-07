import { useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { useAcceptInvitation } from '@/features/teams/hooks/use-teams'

function AcceptInviteForm({ initialToken }: { initialToken: string }) {
  const navigate = useNavigate()
  const acceptMutation = useAcceptInvitation()
  const [token, setToken] = useState(initialToken)

  const handleAccept = async () => {
    const t = token.trim()
    if (!t) return
    try {
      await acceptMutation.mutateAsync(t)
      navigate('/teams', { replace: true })
    } catch {
      /* */
    }
  }

  return (
    <section className="mx-auto max-w-md">
      <h1 className="text-2xl font-semibold">接受团队邀请</h1>
      <p className="mt-2 text-sm text-muted-foreground">
        粘贴管理员提供的邀请令牌，加入团队。
      </p>
      <Card className="mt-6 space-y-4 p-4">
        <label className="block text-sm">
          <span className="text-muted-foreground">邀请令牌</span>
          <Input
            className="mt-1 font-mono text-xs"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="mg_inv_…"
            autoComplete="off"
          />
        </label>
        <Button
          type="button"
          onClick={() => void handleAccept()}
          disabled={acceptMutation.isPending || !token.trim()}
        >
          {acceptMutation.isPending ? '处理中…' : '接受邀请'}
        </Button>
        {acceptMutation.isError ? (
          <p className="text-sm text-red-600 dark:text-red-400">
            无法接受（令牌无效、过期、用户名不匹配或已使用）。
          </p>
        ) : null}
      </Card>
    </section>
  )
}

export function AcceptInvitePage() {
  const [searchParams] = useSearchParams()
  const initial = searchParams.get('token') ?? ''
  return <AcceptInviteForm key={initial} initialToken={initial} />
}
