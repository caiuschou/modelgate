import { useQuery } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import { useConsoleSessionReady } from '@/hooks/use-console-session-ready'
import type { BillingBalanceResponse, BillingLedgerResponse } from '@/features/billing/types'

export function useBillingBalance() {
  const sessionReady = useConsoleSessionReady()
  return useQuery({
    queryKey: ['billing', 'balance'],
    queryFn: () =>
      apiClient.get(apiPath('api/v1/me/billing/balance')).json<BillingBalanceResponse>(),
    staleTime: 5_000,
    enabled: sessionReady,
  })
}

export function useBillingLedger(kind?: 'deposit' | 'usage_charge') {
  const sessionReady = useConsoleSessionReady()
  const k = kind ?? 'all'
  return useQuery({
    queryKey: ['billing', 'ledger', k],
    queryFn: () => {
      const sp = new URLSearchParams({ limit: '100', offset: '0' })
      if (kind) {
        sp.set('kind', kind)
      }
      return apiClient
        .get(apiPath(`api/v1/me/billing/ledger?${sp.toString()}`))
        .json<BillingLedgerResponse>()
    },
    staleTime: 5_000,
    enabled: sessionReady,
  })
}
