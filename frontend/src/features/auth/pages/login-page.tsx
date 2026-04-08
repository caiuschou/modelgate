import { useState } from 'react'
import { Link, Navigate, useNavigate, useSearchParams } from 'react-router-dom'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { InlineLoginForm } from '@/features/auth/components/inline-login-form'
import { useAuthStore } from '@/stores/auth-store'

const DEFAULT_REDIRECT = '/dashboard'

export function LoginPage() {
  const token = useAuthStore((state) => state.token)
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const passwordChangedBanner = searchParams.get('password_changed') === '1'
  const [usernamePrefill] = useState(() => searchParams.get('username') ?? '')

  if (token) {
    const redirect = searchParams.get('redirect') ?? DEFAULT_REDIRECT
    return <Navigate to={redirect} replace />
  }

  const redirect =
    searchParams.get('redirect') && searchParams.get('redirect') !== ''
      ? (searchParams.get('redirect') as string)
      : DEFAULT_REDIRECT

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
          <p className="text-sm text-muted-foreground">
            <Link
              to="/models"
              className="font-medium text-primary underline-offset-4 hover:underline"
            >
              浏览 OpenRouter 模型目录
            </Link>
            （无需登录）
          </p>
        </CardHeader>
        <CardContent>
          {passwordChangedBanner ? (
            <p className="mb-4 text-sm text-muted-foreground" role="status">
              密码已更新，请使用新密码登录。
            </p>
          ) : null}
          <InlineLoginForm
            initialUsername={usernamePrefill}
            showAuxLinks={false}
            onLoggedIn={() => navigate(redirect, { replace: true })}
          />
        </CardContent>
      </Card>
    </main>
  )
}
