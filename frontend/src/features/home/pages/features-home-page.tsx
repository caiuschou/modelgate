import { Link } from 'react-router-dom'
import { Card } from '@/components/ui/card'

const highlights = [
  {
    title: 'OpenAI 兼容网关',
    body: '统一接入 Chat Completions，支持流式响应与上游转发策略。',
  },
  {
    title: '审计与日志',
    body: '按时间、模型、状态筛选请求记录，支持导出与详情查看。',
  },
  {
    title: 'BYOK',
    body: '自带上游密钥与 Base URL，网关侧加密存储，审计可区分平台与 BYOK。',
  },
  {
    title: '团队与密钥',
    body: '个人 / 团队空间、API 密钥与模型白名单等控制台能力。',
  },
  {
    title: 'OpenRouter 模型目录',
    body: '内置 OpenRouter 公共模型列表，便于选型与复制模型 id。',
  },
  {
    title: '统计分析',
    body: '用量与错误分布等仪表盘（持续接入真实指标）。',
  },
]

export function FeaturesHomePage() {
  return (
    <div className="space-y-10">
      <section className="rounded-xl border border-border bg-gradient-to-br from-muted/60 to-card p-8 shadow-sm">
        <h1 className="text-3xl font-semibold tracking-tight">ModelGate</h1>
        <p className="mt-3 max-w-2xl text-base text-muted-foreground">
          面向团队与个人的 LLM 网关控制台：密钥、BYOK、审计与 OpenRouter
          模型发现，一套界面管理调用与可观测性。
        </p>
        <div className="mt-6 flex flex-wrap gap-3">
          <p className="text-sm text-muted-foreground">
            已有账号？请使用页面右上角{' '}
            <span className="font-medium text-foreground">登录</span>（弹窗）。尚无账号可{' '}
            <Link
              to="/register"
              className="font-medium text-primary underline-offset-4 hover:underline"
            >
              注册
            </Link>
            。
          </p>
          <Link
            to="/models"
            className="inline-flex h-10 items-center rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-accent"
          >
            浏览模型目录
          </Link>
        </div>
      </section>

      <section>
        <h2 className="text-lg font-semibold">产品特性</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          以下为当前控制台已覆盖或规划中的能力摘要。
        </p>
        <div className="mt-4 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {highlights.map((item) => (
            <Card key={item.title} className="p-4">
              <h3 className="font-medium">{item.title}</h3>
              <p className="mt-2 text-sm text-muted-foreground">{item.body}</p>
            </Card>
          ))}
        </div>
      </section>
    </div>
  )
}
