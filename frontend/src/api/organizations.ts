import { del, get, post } from './client'
import type { AddOrgMemberParams } from '../bindings/AddOrgMemberParams'
import type { CreateOrganizationParams } from '../bindings/CreateOrganizationParams'
import type { CreateTeamParams } from '../bindings/CreateTeamParams'
import type { GrantTeamRepoParams } from '../bindings/GrantTeamRepoParams'
import type { OrganizationResponse } from '../bindings/OrganizationResponse'
import type { OrgMemberResponse } from '../bindings/OrgMemberResponse'
import type { TeamMemberResponse } from '../bindings/TeamMemberResponse'
import type { TeamRepoResponse } from '../bindings/TeamRepoResponse'
import type { TeamResponse } from '../bindings/TeamResponse'

function orgPath(handle: string): string {
  return `/api/organizations/${encodeURIComponent(handle)}`
}

export const organizationsApi = {
  list: () => get<OrganizationResponse[]>('/api/organizations'),
  show: (handle: string) => get<OrganizationResponse>(orgPath(handle)),
  create: (params: CreateOrganizationParams) =>
    post<OrganizationResponse>('/api/organizations', params),

  members: (handle: string) => get<OrgMemberResponse[]>(`${orgPath(handle)}/members`),
  addMember: (handle: string, params: AddOrgMemberParams) =>
    post<OrgMemberResponse>(`${orgPath(handle)}/members`, params),
  removeMember: (handle: string, userId: number) =>
    del(`${orgPath(handle)}/members/${userId}`),

  teams: (handle: string) => get<TeamResponse[]>(`${orgPath(handle)}/teams`),
  createTeam: (handle: string, params: CreateTeamParams) =>
    post<TeamResponse>(`${orgPath(handle)}/teams`, params),

  teamMembers: (handle: string, teamId: number) =>
    get<TeamMemberResponse[]>(`${orgPath(handle)}/teams/${teamId}/members`),
  addTeamMember: (handle: string, teamId: number, params: AddOrgMemberParams) =>
    post<TeamMemberResponse>(`${orgPath(handle)}/teams/${teamId}/members`, params),
  removeTeamMember: (handle: string, teamId: number, userId: number) =>
    del(`${orgPath(handle)}/teams/${teamId}/members/${userId}`),

  teamRepos: (handle: string, teamId: number) =>
    get<TeamRepoResponse[]>(`${orgPath(handle)}/teams/${teamId}/repositories`),
  grantRepo: (handle: string, teamId: number, params: GrantTeamRepoParams) =>
    post<TeamRepoResponse>(`${orgPath(handle)}/teams/${teamId}/repositories`, params),
  revokeRepo: (handle: string, teamId: number, repoId: number) =>
    del(`${orgPath(handle)}/teams/${teamId}/repositories/${repoId}`),
}
