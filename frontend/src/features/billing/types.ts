export type BillingBalanceResponse = {
  balance_minor: string
  balance_usd: string
  usd_scale: number
  currency: string
}

export type BillingLedgerItem = {
  id: number
  created_at: number
  kind: string
  amount_minor: string
  amount_usd: string
  balance_after_minor: string
  balance_after_usd: string
  request_id: string | null
  model: string | null
  prompt_tokens: number | null
  completion_tokens: number | null
  external_ref: string | null
}

export type BillingLedgerResponse = {
  data: BillingLedgerItem[]
}

export type AdminDepositResponse = {
  user_id: number
  username: string
  balance_minor: string
  balance_usd: string
  usd_scale: number
  currency: string
}
