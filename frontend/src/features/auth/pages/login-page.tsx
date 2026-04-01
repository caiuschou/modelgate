import type { FormEvent } from 'react'
import { useState } from 'react'
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { useAuthStore } from '@/stores/auth-store'

export function LoginPage() {
  const [username, setUsername] = useState('admin')
  const token = useAuthStore((state) => state.token)
  const login = useAuthStore((state) => state.login)
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()

  if (token) {
    const redirect = searchParams.get('redirect')
    return <Navigate to={redirect ?? '/'} replace />
  }

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const role = username.trim().toLowerCase() === 'admin' ? 'admin' : 'user'
    login('dev-token', { username: username.trim() || 'guest', role })
    const redirect = searchParams.get('redirect')
    navigate(redirect ?? '/')
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>登录 ModelGate</CardTitle>
          <p className="text-sm text-muted-foreground">
            当前为开发模式，输入 admin 将获得管理员权限。
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <label className="block text-sm font-medium">
              用户名
              <Input
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                className="mt-1"
                placeholder="请输入用户名"
              />
            </label>

            <Button type="submit" className="w-full">
              登录
            </Button>
          </form>
        </CardContent>
      </Card>
    </main>
  )
}
