import type { FormEvent } from 'react'
import { useEffect, useId, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Modal } from '@/components/ui/modal'
import { apiPath } from '@/lib/api-client'
import { useAuthStore } from '@/stores/auth-store'
import { useTeamStore } from '@/stores/team-store'

type ChangePasswordModalProps = {
  open: boolean
  onClose: () => void
}

export function ChangePasswordModal({ open, onClose }: ChangePasswordModalProps) {
  const user = useAuthStore((s) => s.user)
  const logout = useAuthStore((s) => s.logout)
  const descId = useId()
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [formError, setFormError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    if (!open) {
      setNewPassword('')
      setConfirmPassword('')
      setFormError(null)
      setSubmitting(false)
    }
  }, [open])

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setFormError(null)
    if (newPassword.length < 8) {
      setFormError('新密码至少需要 8 个字符')
      return
    }
    if (newPassword !== confirmPassword) {
      setFormError('两次输入的新密码不一致')
      return
    }

    setSubmitting(true)
    try {
      const token = useAuthStore.getState().token
      if (!token) {
        setFormError('登录状态已失效，请重新登录')
        return
      }
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      }
      const teamId = useTeamStore.getState().currentTeamId
      if (teamId != null) {
        headers['X-Team-Id'] = String(teamId)
      }
      const res = await fetch(apiPath('/api/v1/me/password'), {
        method: 'POST',
        headers,
        body: JSON.stringify({ new_password: newPassword }),
      })
      const text = await res.text()
      if (!res.ok) {
        try {
          const body = JSON.parse(text) as { error?: { message?: string } }
          setFormError(body.error?.message ?? '修改失败，请稍后重试')
        } catch {
          setFormError('修改失败，请稍后重试')
        }
        return
      }
      const u = user?.username ?? ''
      logout()
      const next = new URLSearchParams()
      if (u) {
        next.set('username', u)
      }
      next.set('password_changed', '1')
      window.location.assign(`/login?${next.toString()}`)
    } catch {
      setFormError('网络错误，请稍后重试')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Modal
      open={open}
      onClose={() => {
        if (!submitting) {
          onClose()
        }
      }}
      title="修改密码"
      descriptionId={descId}
    >
      <p id={descId} className="mb-4 text-sm text-muted-foreground">
        修改成功后将退出登录，请使用新密码重新登录。
      </p>
      <form onSubmit={handleSubmit} className="space-y-4" noValidate>
        {formError ? (
          <p className="text-sm text-destructive" role="alert">
            {formError}
          </p>
        ) : null}
        <label className="block text-sm font-medium">
          新密码
          <Input
            name="new_password"
            type="password"
            autoComplete="new-password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            className="mt-1"
            disabled={submitting}
          />
        </label>
        <label className="block text-sm font-medium">
          确认新密码
          <Input
            name="confirm_password"
            type="password"
            autoComplete="new-password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            className="mt-1"
            disabled={submitting}
          />
        </label>
        <div className="flex flex-wrap justify-end gap-2 pt-2">
          <Button
            type="button"
            variant="outline"
            disabled={submitting}
            onClick={onClose}
          >
            取消
          </Button>
          <Button type="submit" disabled={submitting}>
            {submitting ? '提交中…' : '保存新密码'}
          </Button>
        </div>
      </form>
    </Modal>
  )
}
