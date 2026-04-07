import type { FormEvent } from 'react'
import { useEffect, useState } from 'react'
import { flushSync } from 'react-dom'
import { Link, Navigate, useNavigate, useSearchParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { apiPath } from '@/lib/api-client'
import { useAuthStore } from '@/stores/auth-store'

type LoginResponse = {
  token: string
  user: { username: string; role: string }
}

export function LoginPage() {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [formError, setFormError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const token = useAuthStore((state) => state.token)
  const login = useAuthStore((state) => state.login)
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()

  useEffect(() => {
    const prefill = searchParams.get('username')
    if (prefill) {
      setUsername(prefill)
    }
  }, [searchParams])

  if (token) {
    const redirect = searchParams.get('redirect')
    return <Navigate to={redirect ?? '/'} replace />
  }

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setFormError(null)
    const u = username.trim()

    setSubmitting(true)
    try {
      const res = await fetch(apiPath('/api/v1/auth/login'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: u, password }),
      })
      const text = await res.text()
      if (!res.ok) {
        try {
          const body = JSON.parse(text) as { error?: { message?: string } }
          setFormError(
            body.error?.message ?? '登录失败，请检查用户名和密码',
          )
        } catch {
          setFormError('登录失败，请检查用户名和密码')
        }
        return
      }
      const data = JSON.parse(text) as LoginResponse
      const role = data.user.role === 'admin' ? 'admin' : 'user'
      flushSync(() => {
        login(data.token, { username: data.user.username, role })
      })
      navigate(searchParams.get('redirect') ?? '/', { replace: true })
    } catch {
      setFormError('网络错误，请稍后重试')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>登录 ModelGate</CardTitle>
          <p className="text-sm text-muted-foreground">
            使用注册时的用户名与密码登录。没有账号？
            <Link
              to="/register"
              className="ml-1 font-medium text-primary underline-offset-4 hover:underline"
            >
              去注册
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
                autoComplete="current-password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                className="mt-1"
                placeholder="请输入密码"
                disabled={submitting}
              />
            </label>
            <Button type="submit" className="w-full" disabled={submitting}>
              {submitting ? '登录中…' : '登录'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </main>
  )
}
