import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import type { ApiKeyListResponse, CreateMyApiKeyResponse } from '@/features/api-keys/types'

export function useMyApiKeys() {
  return useQuery({
    queryKey: ['api-keys', 'mine'],
    queryFn: () =>
      apiClient.get(apiPath('api/v1/me/api-keys')).json<ApiKeyListResponse>(),
    staleTime: 10_000,
  })
}

export function useCreateMyApiKey() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () =>
      apiClient.post(apiPath('api/v1/me/api-keys')).json<CreateMyApiKeyResponse>(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['api-keys', 'mine'] })
    },
  })
}

export function useRevokeMyApiKey() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (keyId: number) =>
      apiClient.post(apiPath(`api/v1/me/api-keys/${keyId}/revoke`)),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['api-keys', 'mine'] })
    },
  })
}
