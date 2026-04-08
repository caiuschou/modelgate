import { useQuery } from '@tanstack/react-query'
import { fetchOpenRouterModels } from '@/features/models/lib/openrouter-models'

const STALE_MS = 10 * 60 * 1000

export function useOpenRouterModels() {
  return useQuery({
    queryKey: ['openrouter', 'models', 'all'],
    queryFn: fetchOpenRouterModels,
    staleTime: STALE_MS,
  })
}
