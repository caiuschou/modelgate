import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient, apiPath } from '@/lib/api-client'
import { useConsoleSessionReady } from '@/hooks/use-console-session-ready'
import type {
  CreateTeamResponse,
  InviteCreatedResponse,
  MemberListResponse,
  RegisterMemberOnBehalfResponse,
  Team,
  TeamListResponse,
} from '@/features/teams/types'

export function useMyTeams() {
  const ready = useConsoleSessionReady()
  return useQuery({
    queryKey: ['teams', 'list'],
    queryFn: () =>
      apiClient.get(apiPath('api/v1/teams')).json<TeamListResponse>(),
    staleTime: 15_000,
    enabled: ready,
  })
}

export function useTeam(teamId: number | undefined) {
  const ready = useConsoleSessionReady()
  return useQuery({
    queryKey: ['teams', 'one', teamId],
    queryFn: () =>
      apiClient.get(apiPath(`api/v1/teams/${teamId}`)).json<Team>(),
    enabled: ready && teamId !== undefined && teamId > 0,
    staleTime: 15_000,
  })
}

export function useTeamMembers(teamId: number | undefined) {
  const ready = useConsoleSessionReady()
  return useQuery({
    queryKey: ['teams', 'members', teamId],
    queryFn: () =>
      apiClient
        .get(apiPath(`api/v1/teams/${teamId}/members`))
        .json<MemberListResponse>(),
    enabled: ready && teamId !== undefined && teamId > 0,
    staleTime: 10_000,
  })
}

export function useCreateTeam() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: { name: string; slug: string }) =>
      apiClient
        .post(apiPath('api/v1/teams'), { json: body })
        .json<CreateTeamResponse>(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['teams'] })
    },
  })
}

export function useDeleteTeam() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (teamId: number) =>
      apiClient.delete(apiPath(`api/v1/teams/${teamId}`)),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['teams'] })
    },
  })
}

export function useCreateInvitation(teamId: number) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: { invitee_username: string; role: string }) =>
      apiClient
        .post(apiPath(`api/v1/teams/${teamId}/invitations`), { json: body })
        .json<InviteCreatedResponse>(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['teams', 'members', teamId] })
    },
  })
}

export function useRegisterMemberOnBehalf(teamId: number) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: { username: string; password: string; role: string }) =>
      apiClient
        .post(
          apiPath(`api/v1/teams/${teamId}/members/register-on-behalf`),
          { json: body },
        )
        .json<RegisterMemberOnBehalfResponse>(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['teams', 'members', teamId] })
    },
  })
}

export function usePatchTeamMember(teamId: number) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({
      userId,
      role,
    }: {
      userId: number
      role: string
    }) =>
      apiClient.patch(
        apiPath(`api/v1/teams/${teamId}/members/${userId}`),
        { json: { role } },
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['teams', 'members', teamId] })
    },
  })
}

export function useRemoveTeamMember(teamId: number) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (userId: number) =>
      apiClient.delete(
        apiPath(`api/v1/teams/${teamId}/members/${userId}`),
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['teams', 'members', teamId] })
    },
  })
}

export function useAcceptInvitation() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (token: string) =>
      apiClient
        .post(apiPath('api/v1/invitations/accept'), { json: { token } })
        .json<{ team_id: number; role: string }>(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['teams'] })
    },
  })
}

export function useDeleteInvitation(teamId: number) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (invitationId: number) =>
      apiClient.delete(
        apiPath(
          `api/v1/teams/${teamId}/invitations/${invitationId}`,
        ),
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['teams'] })
    },
  })
}
