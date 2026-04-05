import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import type {
  ApiKeyListResponse,
  ApiKeySummary,
  CreateMyApiKeyBody,
  CreateMyApiKeyResponse,
} from '@/features/api-keys/types'

export function useMyApiKeys() {
  return useQuery({
    queryKey: ['api-keys', 'mine'],
    queryFn: () =>
      apiClient.get(apiPath('api/v1/me/api-keys')).json<ApiKeyListResponse>(),
    staleTime: 10_000,
  })
}

export function useMyApiKey(id: number | undefined) {
  return useQuery({
    queryKey: ['api-keys', 'one', id],
    queryFn: () =>
      apiClient.get(apiPath(`api/v1/me/api-keys/${id}`)).json<ApiKeySummary>(),
    enabled: id !== undefined && id > 0,
    staleTime: 10_000,
  })
}

export function useCreateMyApiKey() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateMyApiKeyBody) =>
      apiClient
        .post(apiPath('api/v1/me/api-keys'), { json: body })
        .json<CreateMyApiKeyResponse>(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['api-keys'] })
    },
  })
}

export function usePatchMyApiKey() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({
      id,
      body,
    }: {
      id: number
      body: Record<string, unknown>
    }) =>
      apiClient.patch(apiPath(`api/v1/me/api-keys/${id}`), { json: body }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['api-keys'] })
    },
  })
}

export function useRevokeMyApiKey() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (keyId: number) =>
      apiClient.post(apiPath(`api/v1/me/api-keys/${keyId}/revoke`)),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['api-keys'] })
    },
  })
}
