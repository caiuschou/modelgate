import { useState } from 'react'
import { Link } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/shared/empty-state'
import { useCreateTeam, useMyTeams } from '@/features/teams/hooks/use-teams'
import { useTeamStore } from '@/stores/team-store'

export function TeamsPage() {
  const { data, isLoading, isError, refetch } = useMyTeams()
  const createMutation = useCreateTeam()
  const setTeamContext = useTeamStore((s) => s.setTeamContext)
  const [showForm, setShowForm] = useState(false)
  const [name, setName] = useState('')
  const [slug, setSlug] = useState('')

  const handleCreate = async () => {
    const n = name.trim()
    const s = slug.trim().toLowerCase()
    if (!n || !s) return
    try {
      const res = await createMutation.mutateAsync({ name: n, slug: s })
      setTeamContext(res.team.id)
      setShowForm(false)
      setName('')
      setSlug('')
    } catch {
      /* ky */
    }
  }

  const teams = data?.data ?? []

  return (
    <section>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold">团队</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            创建或加入团队后，可在顶部切换工作空间，管理团队 API 密钥与审计视图。
          </p>
        </div>
        <Button
          variant="outline"
          onClick={() => setShowForm((v) => !v)}
        >
          {showForm ? '取消' : '新建团队'}
        </Button>
      </div>

      {showForm ? (
        <Card className="mt-6 space-y-3 p-4">
          <p className="text-sm font-medium">新建团队</p>
          <label className="block text-sm">
            <span className="text-muted-foreground">名称</span>
            <Input
              className="mt-1"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如：平台组"
            />
          </label>
          <label className="block text-sm">
            <span className="text-muted-foreground">Slug（URL，小写与连字符）</span>
            <Input
              className="mt-1"
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="例如：platform"
            />
          </label>
          <Button
            onClick={() => void handleCreate()}
            disabled={createMutation.isPending || !name.trim() || !slug.trim()}
          >
            {createMutation.isPending ? '创建中…' : '创建'}
          </Button>
          {createMutation.isError ? (
            <p className="text-sm text-red-600 dark:text-red-400">
              创建失败（名称或 slug 重复，或网络错误）。
            </p>
          ) : null}
        </Card>
      ) : null}

      <div className="mt-8">
        {isLoading ? (
          <p className="text-sm text-muted-foreground">加载中…</p>
        ) : isError ? (
          <div className="space-y-2">
            <p className="text-sm text-red-600">加载失败</p>
            <Button size="sm" variant="outline" onClick={() => void refetch()}>
              重试
            </Button>
          </div>
        ) : teams.length === 0 ? (
          <EmptyState
            title="暂无团队"
            description="创建第一个团队以与他人共享 API 密钥与日志视图。"
          />
        ) : (
          <ul className="space-y-3">
            {teams.map((t) => (
              <li key={t.id}>
                <Card className="flex flex-col gap-2 p-4 sm:flex-row sm:items-center sm:justify-between">
                  <div>
                    <p className="font-medium">{t.name}</p>
                    <p className="text-xs text-muted-foreground">
                      slug: {t.slug} · 我的角色：{t.role}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => setTeamContext(t.id)}
                    >
                      设为当前空间
                    </Button>
                    <Button size="sm" variant="outline" asChild>
                      <Link to={`/teams/${t.id}/members`}>成员</Link>
                    </Button>
                  </div>
                </Card>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  )
}
