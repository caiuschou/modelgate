import { useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  useCreateInvitation,
  useDeleteTeam,
  useMyTeams,
  usePatchTeamMember,
  useRegisterMemberOnBehalf,
  useRemoveTeamMember,
  useTeam,
  useTeamMembers,
} from '@/features/teams/hooks/use-teams'
import { useTeamStore } from '@/stores/team-store'

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

const PASSWORD_ALPHABET =
  'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'

/** URL-safe-ish random string for initial passwords (≥8 chars for API). */
function generateRandomPassword(length = 16): string {
  const bytes = new Uint32Array(length)
  crypto.getRandomValues(bytes)
  let out = ''
  for (let i = 0; i < length; i++) {
    out += PASSWORD_ALPHABET[bytes[i]! % PASSWORD_ALPHABET.length]!
  }
  return out
}

export function TeamMembersPage() {
  const { teamId: tid } = useParams<{ teamId: string }>()
  const teamId = tid ? Number.parseInt(tid, 10) : 0
  const { data: team, isLoading: teamLoading } = useTeam(
    Number.isFinite(teamId) && teamId > 0 ? teamId : undefined,
  )
  const { data: membersRes, isLoading: memLoading } = useTeamMembers(
    Number.isFinite(teamId) && teamId > 0 ? teamId : undefined,
  )
  const inviteMutation = useCreateInvitation(teamId)
  const registerOnBehalfMutation = useRegisterMemberOnBehalf(teamId)
  const deleteTeamMutation = useDeleteTeam()
  const patchRole = usePatchTeamMember(teamId)
  const removeMember = useRemoveTeamMember(teamId)
  const setTeamContext = useTeamStore((s) => s.setTeamContext)
  const { refetch: refetchTeams } = useMyTeams()

  const [inviteUser, setInviteUser] = useState('')
  const [inviteRole, setInviteRole] = useState<'member' | 'admin'>('member')
  const [inviteToken, setInviteToken] = useState<string | null>(null)

  const [provUsername, setProvUsername] = useState('')
  const [provPassword, setProvPassword] = useState('')
  const [provRole, setProvRole] = useState<'member' | 'admin'>('member')
  const [provisionedUser, setProvisionedUser] = useState<string | null>(null)
  const [provPasswordCopyHint, setProvPasswordCopyHint] = useState<
    string | null
  >(null)

  const canManage =
    team?.role === 'owner' || team?.role === 'admin'

  const handleInvite = async () => {
    const u = inviteUser.trim()
    if (!u) return
    setInviteToken(null)
    try {
      const res = await inviteMutation.mutateAsync({
        invitee_username: u,
        role: inviteRole,
      })
      setInviteToken(res.token)
      setInviteUser('')
    } catch {
      /* handled visually */
    }
  }

  const handleCopyProvPassword = async () => {
    const p = provPassword.trim()
    if (!p) return
    try {
      await navigator.clipboard.writeText(p)
      setProvPasswordCopyHint('已复制到剪贴板')
      window.setTimeout(() => setProvPasswordCopyHint(null), 2000)
    } catch {
      setProvPasswordCopyHint('复制失败，请手动全选复制')
      window.setTimeout(() => setProvPasswordCopyHint(null), 3000)
    }
  }

  const handleRegisterOnBehalf = async () => {
    const u = provUsername.trim()
    if (!u || provPassword.length < 8) return
    setProvisionedUser(null)
    setProvPasswordCopyHint(null)
    try {
      await registerOnBehalfMutation.mutateAsync({
        username: u,
        password: provPassword,
        role: provRole,
      })
      setProvisionedUser(u)
      setProvUsername('')
      setProvPassword('')
    } catch {
      /* handled visually */
    }
  }

  const handleDeleteTeam = async () => {
    if (!team) return
    if (
      !window.confirm(
        `确定删除团队「${team.name}」？将级联删除团队密钥等数据，不可恢复。`,
      )
    ) {
      return
    }
    try {
      await deleteTeamMutation.mutateAsync(team.id)
      setTeamContext(null)
      await refetchTeams()
      window.location.href = '/teams'
    } catch {
      /* */
    }
  }

  const members = membersRes?.data ?? []

  if (!Number.isFinite(teamId) || teamId <= 0) {
    return <p className="text-sm text-muted-foreground">无效的团队 ID</p>
  }

  return (
    <section>
      <div className="mb-6 flex flex-wrap items-center gap-2 text-sm">
        <Link to="/teams" className="text-primary hover:underline">
          团队列表
        </Link>
        <span className="text-muted-foreground">/</span>
        <span className="font-medium">{team?.name ?? `团队 ${teamId}`}</span>
      </div>

      {teamLoading ? (
        <p className="text-sm text-muted-foreground">加载团队…</p>
      ) : team ? (
        <div className="mb-6 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-semibold">{team.name}</h1>
            <p className="text-sm text-muted-foreground">
              slug: {team.slug} · 我的角色：{team.role}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="secondary" onClick={() => setTeamContext(team.id)}>
              设为当前空间
            </Button>
            {team.role === 'owner' ? (
              <Button
                size="sm"
                variant="destructive"
                onClick={() => void handleDeleteTeam()}
                disabled={deleteTeamMutation.isPending}
              >
                删除团队
              </Button>
            ) : null}
          </div>
        </div>
      ) : null}

      {canManage ? (
        <Card className="mb-8 space-y-3 p-4">
          <p className="text-sm font-medium">邀请成员</p>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
            <label className="block grow text-sm">
              <span className="text-muted-foreground">用户名（须已注册）</span>
              <Input
                className="mt-1"
                value={inviteUser}
                onChange={(e) => setInviteUser(e.target.value)}
                placeholder="同事的用户名"
              />
            </label>
            <label className="block text-sm sm:w-40">
              <span className="text-muted-foreground">角色</span>
              <select
                className="mt-1 w-full rounded border border-border bg-background px-2 py-2 text-sm"
                value={inviteRole}
                onChange={(e) =>
                  setInviteRole(e.target.value as 'member' | 'admin')
                }
                aria-label="邀请角色"
              >
                <option value="member">member</option>
                <option value="admin">admin</option>
              </select>
            </label>
            <Button
              type="button"
              onClick={() => void handleInvite()}
              disabled={inviteMutation.isPending || !inviteUser.trim()}
            >
              发送邀请
            </Button>
          </div>
          {inviteMutation.isError ? (
            <p className="text-sm text-red-600 dark:text-red-400">
              邀请失败（用户不存在、已是成员或无权限）。
            </p>
          ) : null}
          {inviteToken ? (
            <div className="rounded border border-amber-600/40 bg-amber-500/5 p-3 text-sm">
              <p className="font-medium text-amber-900 dark:text-amber-200">
                一次性邀请链接令牌（仅显示本次）
              </p>
              <p className="mt-1 break-all font-mono text-xs">{inviteToken}</p>
              <p className="mt-2 text-xs text-muted-foreground">
                让对方在已登录状态下打开「接受邀请」页并粘贴令牌，或使用你方渠道安全传递。
              </p>
            </div>
          ) : null}
        </Card>
      ) : null}

      {canManage ? (
        <Card className="mb-8 space-y-3 p-4">
          <p className="text-sm font-medium">代为注册</p>
          <p className="text-xs text-muted-foreground">
            为尚未有账号的同事创建控制台账户并加入本团队，无需对方持有平台邀请码。初始密码请通过安全渠道告知对方，并建议其首次登录后修改。
          </p>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
            <label className="block grow text-sm">
              <span className="text-muted-foreground">新用户名</span>
              <Input
                className="mt-1"
                value={provUsername}
                onChange={(e) => setProvUsername(e.target.value)}
                placeholder="全新用户名（不可与已有账号重复）"
                autoComplete="off"
              />
            </label>
            <label className="block grow text-sm sm:min-w-[200px]">
              <span className="text-muted-foreground">初始密码（至少 8 位）</span>
              <p className="mt-0.5 text-xs text-muted-foreground">
                明文显示，便于复制后通过安全渠道发给对方；请勿在公开场合展示。
              </p>
              <div className="mt-1 flex flex-wrap gap-2">
                <Input
                  className="min-w-[12rem] flex-1 font-mono text-sm"
                  type="text"
                  value={provPassword}
                  onChange={(e) => setProvPassword(e.target.value)}
                  placeholder="代为设置初始密码"
                  autoComplete="off"
                  spellCheck={false}
                />
                <Button
                  type="button"
                  variant="outline"
                  className="shrink-0 whitespace-nowrap"
                  onClick={() => setProvPassword(generateRandomPassword())}
                  aria-label="生成随机初始密码"
                >
                  随机生成
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  className="shrink-0 whitespace-nowrap"
                  onClick={() => void handleCopyProvPassword()}
                  disabled={!provPassword.trim()}
                  aria-label="复制初始密码"
                >
                  复制密码
                </Button>
              </div>
              {provPasswordCopyHint ? (
                <p className="mt-1 text-xs text-muted-foreground">
                  {provPasswordCopyHint}
                </p>
              ) : null}
            </label>
            <label className="block text-sm sm:w-40">
              <span className="text-muted-foreground">角色</span>
              <select
                className="mt-1 w-full rounded border border-border bg-background px-2 py-2 text-sm"
                value={provRole}
                onChange={(e) =>
                  setProvRole(e.target.value as 'member' | 'admin')
                }
                aria-label="代为注册角色"
              >
                <option value="member">member</option>
                <option value="admin">admin</option>
              </select>
            </label>
            <Button
              type="button"
              onClick={() => void handleRegisterOnBehalf()}
              disabled={
                registerOnBehalfMutation.isPending ||
                !provUsername.trim() ||
                provPassword.length < 8
              }
            >
              创建并加入团队
            </Button>
          </div>
          {registerOnBehalfMutation.isError ? (
            <p className="text-sm text-red-600 dark:text-red-400">
              失败：用户名已存在、权限不足或网络错误。
            </p>
          ) : null}
          {provisionedUser ? (
            <p className="text-sm text-emerald-800 dark:text-emerald-200" role="status">
              已创建账户「{provisionedUser}」并加入团队，请将初始密码告知对方。
            </p>
          ) : null}
        </Card>
      ) : null}

      <h2 className="text-lg font-medium">成员</h2>
      {memLoading ? (
        <p className="mt-2 text-sm text-muted-foreground">加载成员…</p>
      ) : (
        <div className="mt-3 overflow-x-auto rounded border border-border">
          <table className="w-full min-w-[480px] text-left text-sm">
            <thead className="border-b border-border bg-muted/50">
              <tr>
                <th className="p-3 font-medium">用户</th>
                <th className="p-3 font-medium">角色</th>
                <th className="p-3 font-medium">加入时间</th>
                {canManage ? (
                  <th className="p-3 font-medium">操作</th>
                ) : null}
              </tr>
            </thead>
            <tbody>
              {members.map((m) => (
                <tr key={m.user_id} className="border-b border-border last:border-0">
                  <td className="p-3">{m.username}</td>
                  <td className="p-3">{m.role}</td>
                  <td className="p-3 text-muted-foreground">
                    {formatTime(m.joined_at)}
                  </td>
                  {canManage ? (
                    <td className="p-3">
                      {m.role !== 'owner' ? (
                        <div className="flex flex-wrap gap-1">
                          {m.role === 'member' ? (
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() =>
                                void patchRole.mutateAsync({
                                  userId: m.user_id,
                                  role: 'admin',
                                })
                              }
                            >
                              升为 admin
                            </Button>
                          ) : (
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() =>
                                void patchRole.mutateAsync({
                                  userId: m.user_id,
                                  role: 'member',
                                })
                              }
                            >
                              降为 member
                            </Button>
                          )}
                          <Button
                            size="sm"
                            variant="ghost"
                            className="text-red-600"
                            onClick={() => {
                              if (
                                window.confirm(
                                  `将 ${m.username} 移出团队？`,
                                )
                              ) {
                                void removeMember.mutateAsync(m.user_id)
                              }
                            }}
                          >
                            移除
                          </Button>
                        </div>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </td>
                  ) : null}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}
