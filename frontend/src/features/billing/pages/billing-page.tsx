import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { EmptyState } from '@/components/shared/empty-state'
import { useBillingBalance, useBillingLedger } from '@/features/billing/hooks/use-billing'

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

type TabId = 'topup' | 'usage' | 'deposits'

export function BillingPage() {
  const [tab, setTab] = useState<TabId>('topup')
  const { data: bal, isLoading: balLoading } = useBillingBalance()
  const usageLedger = useBillingLedger('usage_charge')
  const depositLedger = useBillingLedger('deposit')

  return (
    <div className="mx-auto max-w-5xl space-y-6 p-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">充值中心</h1>
        <p className="text-muted-foreground mt-1 text-sm">
          账户余额以 USD 计价（整数 minor，scale 见接口）；扣费金额与上游返回的 cost / cost_details 一致。
        </p>
      </div>

      <Card className="p-6">
        <div className="flex flex-wrap items-baseline justify-between gap-4">
          <div>
            <p className="text-muted-foreground text-sm">当前余额</p>
            <p className="font-mono text-3xl font-semibold tabular-nums">
              {balLoading ? '…' : (bal?.balance_usd ?? '0')}
            </p>
            <p className="text-muted-foreground mt-1 text-xs">
              {bal?.currency ?? 'USD'} · minor scale {bal?.usd_scale ?? 15}
            </p>
          </div>
        </div>
      </Card>

      <div className="flex flex-wrap gap-2 border-b border-border pb-2">
        {(
          [
            ['topup', '在线充值'],
            ['usage', '消费记录'],
            ['deposits', '充值记录'],
          ] as const
        ).map(([id, label]) => (
          <Button
            key={id}
            type="button"
            variant={tab === id ? 'default' : 'ghost'}
            size="sm"
            onClick={() => setTab(id)}
          >
            {label}
          </Button>
        ))}
      </div>

      {tab === 'topup' && (
        <Card className="p-6 space-y-4">
          <p className="text-muted-foreground text-sm">
            在线支付渠道接入前，账户充值由运营通过管理入口完成。最低充值金额与币种规则以服务端{' '}
            <code className="text-xs">billing.min_deposit_cents</code> 为准。
          </p>
        </Card>
      )}

      {tab === 'usage' && (
        <LedgerTable
          title="消费记录"
          query={usageLedger}
          emptyHint="暂无模型调用扣费记录。"
        />
      )}

      {tab === 'deposits' && (
        <LedgerTable
          title="充值记录"
          query={depositLedger}
          emptyHint="暂无充值记录。"
        />
      )}

      {tab === 'usage' && usageLedger.isError && (
        <p className="text-destructive text-sm">加载失败，请稍后重试。</p>
      )}
      {tab === 'deposits' && depositLedger.isError && (
        <p className="text-destructive text-sm">加载失败，请稍后重试。</p>
      )}
    </div>
  )
}

function LedgerTable({
  title,
  query,
  emptyHint,
}: {
  title: string
  query: ReturnType<typeof useBillingLedger>
  emptyHint: string
}) {
  const { data, isLoading } = query
  const rows = data?.data ?? []

  return (
    <Card className="p-6">
      <h2 className="mb-4 text-lg font-medium">{title}</h2>
      {isLoading ? (
        <p className="text-muted-foreground text-sm">加载中…</p>
      ) : rows.length === 0 ? (
        <EmptyState title="无记录" description={emptyHint} />
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-muted-foreground">
                <th className="pb-2 pr-4 font-medium">时间</th>
                <th className="pb-2 pr-4 font-medium">金额</th>
                <th className="pb-2 pr-4 font-medium">余额后</th>
                <th className="pb-2 font-medium">说明</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.id} className="border-b border-border/60">
                  <td className="py-2 pr-4 whitespace-nowrap">{formatTime(r.created_at)}</td>
                  <td className="py-2 pr-4 font-mono tabular-nums">{r.amount_usd}</td>
                  <td className="py-2 pr-4 font-mono tabular-nums text-muted-foreground">
                    {r.balance_after_usd}
                  </td>
                  <td className="py-2 text-muted-foreground">
                    {r.kind === 'usage_charge' && r.model ? (
                      <span>
                        {r.model}
                        {r.request_id ? (
                          <span className="ml-2 font-mono text-xs">({r.request_id.slice(0, 12)}…)</span>
                        ) : null}
                      </span>
                    ) : (
                      r.external_ref ?? r.kind
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Card>
  )
}
