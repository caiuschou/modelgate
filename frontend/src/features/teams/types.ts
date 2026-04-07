export interface Team {
  id: number
  name: string
  slug: string
  created_by_user_id: number
  created_at: number
  role: string
}

export interface TeamListResponse {
  data: Team[]
}

export interface CreateTeamResponse {
  team: Team
}

export interface TeamMember {
  user_id: number
  username: string
  role: string
  joined_at: number
}

export interface MemberListResponse {
  data: TeamMember[]
}

export interface InviteCreatedResponse {
  id: number
  team_id: number
  invitee_username: string
  role: string
  expires_at: number
  token: string
}

export interface RegisterMemberOnBehalfResponse {
  user_id: number
  username: string
}
