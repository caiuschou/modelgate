import type { FormEvent } from 'react'
import { useState } from 'react'
import { HTTPError } from 'ky'
import { Link, Navigate, useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { apiPath, publicApi } from '@/lib/api-client'
import { useAuthStore } from '@/stores/auth-store'

export function RegisterPage() {
  const token = useAuthStore((state) => state.token)
  const navigate = useNavigate()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [inviteCode, setInviteCode] = useState('')
  const [formError, setFormError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  if (token) {
    return <Navigate to="/" replace />
  }

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setFormError(null)

    const u = username.trim()

    setSubmitting(true)
    try {
      await publicApi.post(apiPath('/api/v1/auth/register'), {
        json: {
          username: u,
          password,
          invite_code: inviteCode.trim(),
        },
      })
      navigate(`/login?username=${encodeURIComponent(u)}`, { replace: true })
    } catch (err) {
      if (err instanceof HTTPError) {
        try {
          const body = (await err.response.json()) as {
            error?: { message?: string }
          }
          const msg = body.error?.message
          setFormError(msg ?? '注册失败，请稍后重试')
        } catch {
          setFormError('注册失败，请稍后重试')
        }
      } else {
        setFormError('网络错误，请稍后重试')
      }
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>创建账号</CardTitle>
          <p className="text-sm text-muted-foreground">
            内测注册需填写有效邀请码。已有账号？
            <Link to="/login" className="ml-1 font-medium text-primary underline-offset-4 hover:underline">
              去登录
            </Link>
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4" noValidate>
            {formError ? (
              <p className="text-sm text-destructive" role="alert">
                {formError}
              </p>
            ) : null}
            <label className="block text-sm font-medium">
              用户名
              <Input
                name="username"
                autoComplete="username"
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                className="mt-1"
                placeholder="请输入用户名"
                disabled={submitting}
              />
            </label>
            <label className="block text-sm font-medium">
              密码
              <Input
                name="password"
                type="password"
                autoComplete="new-password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                className="mt-1"
                placeholder="请输入密码"
                disabled={submitting}
              />
            </label>
            <label className="block text-sm font-medium">
              邀请码
              <Input
                name="invite_code"
                autoComplete="off"
                value={inviteCode}
                onChange={(event) => setInviteCode(event.target.value)}
                className="mt-1"
                placeholder="内测邀请码"
                disabled={submitting}
              />
            </label>
            <Button type="submit" className="w-full" disabled={submitting}>
              {submitting ? '提交中…' : '注册'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </main>
  )
}
