const stats = [
  { title: '今日请求数', value: '1,284', hint: '+12.4%' },
  { title: '成功率', value: '99.2%', hint: '+0.3%' },
  { title: '总 Token', value: '2.4M', hint: '+8.1%' },
  { title: '预计成本', value: '$128.40', hint: '-2.6%' },
]

export function DashboardPage() {
  return (
    <section>
      <h1 className="text-2xl font-semibold">仪表盘</h1>
      <p className="mt-1 text-sm text-muted-foreground">
        前端第一阶段骨架已完成，后续接入真实统计接口。
      </p>

      <div className="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {stats.map((item) => (
          <article
            key={item.title}
            className="rounded-lg border border-border bg-card p-4 shadow-sm"
          >
            <p className="text-sm text-muted-foreground">{item.title}</p>
            <p className="mt-2 text-2xl font-semibold">{item.value}</p>
            <p className="mt-1 text-xs text-muted-foreground">{item.hint}</p>
          </article>
        ))}
      </div>
    </section>
  )
}
