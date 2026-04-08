import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import { useConsoleSessionReady } from '@/hooks/use-console-session-ready'
import type {
  ByokListResponse,
  ByokProfileSummary,
  CreateByokBody,
  CreateByokResponse,
} from '@/features/byok/types'
import { useTeamStore } from '@/stores/team-store'

export function useMyByokProfiles() {
  const sessionReady = useConsoleSessionReady()
  const teamId = useTeamStore((s) => s.currentTeamId)
  return useQuery({
    queryKey: ['byok-profiles', 'mine', teamId ?? 'personal'],
    queryFn: () =>
      apiClient.get(apiPath('api/v1/me/byok-profiles')).json<ByokListResponse>(),
    staleTime: 10_000,
    enabled: sessionReady,
  })
}

export function useMyByokProfile(id: number | undefined) {
  const sessionReady = useConsoleSessionReady()
  const teamId = useTeamStore((s) => s.currentTeamId)
  return useQuery({
    queryKey: ['byok-profiles', 'one', id, teamId ?? 'personal'],
    queryFn: () =>
      apiClient
        .get(apiPath(`api/v1/me/byok-profiles/${id}`))
        .json<ByokProfileSummary>(),
    enabled: sessionReady && id !== undefined && id > 0,
    staleTime: 10_000,
  })
}

export function useCreateByokProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateByokBody) =>
      apiClient
        .post(apiPath('api/v1/me/byok-profiles'), { json: body })
        .json<CreateByokResponse>(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['byok-profiles'] })
    },
  })
}

export function usePatchByokProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({
      id,
      body,
    }: {
      id: number
      body: Record<string, unknown>
    }) =>
      apiClient.patch(apiPath(`api/v1/me/byok-profiles/${id}`), { json: body }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['byok-profiles'] })
    },
  })
}

export function useRevokeByokProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (profileId: number) =>
      apiClient.post(apiPath(`api/v1/me/byok-profiles/${profileId}/revoke`)),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['byok-profiles'] })
    },
  })
}
