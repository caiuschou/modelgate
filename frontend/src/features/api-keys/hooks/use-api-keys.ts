import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import { useConsoleSessionReady } from '@/hooks/use-console-session-ready'
import type {
  ApiKeyListResponse,
  ApiKeySummary,
  CreateMyApiKeyBody,
  CreateMyApiKeyResponse,
} from '@/features/api-keys/types'
import { useTeamStore } from '@/stores/team-store'

function teamScopeKey(): string {
  const id = useTeamStore.getState().currentTeamId
  return id === null ? 'personal' : `team:${id}`
}

export function useMyApiKeys() {
  const sessionReady = useConsoleSessionReady()
  const teamId = useTeamStore((s) => s.currentTeamId)
  return useQuery({
    queryKey: ['api-keys', 'mine', teamId ?? 'personal'],
    queryFn: () =>
      apiClient.get(apiPath('api/v1/me/api-keys')).json<ApiKeyListResponse>(),
    staleTime: 10_000,
    enabled: sessionReady,
  })
}

export function useMyApiKey(id: number | undefined) {
  const sessionReady = useConsoleSessionReady()
  const teamId = useTeamStore((s) => s.currentTeamId)
  return useQuery({
    queryKey: ['api-keys', 'one', id, teamId ?? 'personal'],
    queryFn: () =>
      apiClient.get(apiPath(`api/v1/me/api-keys/${id}`)).json<ApiKeySummary>(),
    enabled: sessionReady && id !== undefined && id > 0,
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

/** For callers that need stable scope string without subscribing (e.g. callbacks). */
export { teamScopeKey }

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
