import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { OrganizationResponse } from '../bindings/OrganizationResponse'
import type { OrgMemberResponse } from '../bindings/OrgMemberResponse'
import type { TeamMemberResponse } from '../bindings/TeamMemberResponse'
import type { TeamRepoResponse } from '../bindings/TeamRepoResponse'
import type { TeamResponse } from '../bindings/TeamResponse'

export type OrgPart =
  | 'list'
  | 'create'
  | 'show'
  | 'members'
  | 'addMember'
  | 'removeMember'
  | 'teams'
  | 'createTeam'
  | 'teamMembers'
  | 'addTeamMember'
  | 'removeTeamMember'
  | 'teamRepos'
  | 'grantRepo'
  | 'revokeRepo'

export interface OrgRequest {
  part: OrgPart
  key: string
  handle?: string
  teamId?: number
  userId?: number
  repoId?: number
  params?: unknown
}

interface OrganizationsState {
  keys: Record<OrgPart, string>
  loading: Record<OrgPart, boolean>
  error: Record<OrgPart, string | null>
  orgs: OrganizationResponse[]
  current: OrganizationResponse | null
  members: OrgMemberResponse[]
  teams: TeamResponse[]
  teamMembers: Record<number, TeamMemberResponse[]>
  teamRepos: Record<number, TeamRepoResponse[]>
}

const PARTS: OrgPart[] = [
  'list', 'create', 'show', 'members', 'addMember', 'removeMember',
  'teams', 'createTeam', 'teamMembers', 'addTeamMember', 'removeTeamMember',
  'teamRepos', 'grantRepo', 'revokeRepo',
]
const emptyRecord = <T>(value: T): Record<OrgPart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<OrgPart, T>

const initialState: OrganizationsState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  orgs: [],
  current: null,
  members: [],
  teams: [],
  teamMembers: {},
  teamRepos: {},
}

const organizationsSlice = createSlice({
  name: 'organizations',
  initialState,
  reducers: {
    orgRequested(state, action: PayloadAction<OrgRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    orgSucceeded(
      state,
      action: PayloadAction<{ part: OrgPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      switch (part) {
        case 'list':
          state.orgs = data as OrganizationResponse[]
          break
        case 'create':
          state.orgs.push(data as OrganizationResponse)
          state.current = data as OrganizationResponse
          break
        case 'show':
          state.current = data as OrganizationResponse
          break
        case 'members':
          state.members = data as OrgMemberResponse[]
          break
        case 'addMember':
          state.members.push(data as OrgMemberResponse)
          break
        case 'removeMember':
          state.members = state.members.filter(
            (member) => member.user_id !== (data as { user_id: number }).user_id,
          )
          break
        case 'teams':
          state.teams = data as TeamResponse[]
          break
        case 'createTeam':
          state.teams.push(data as TeamResponse)
          break
        case 'teamMembers': {
          const { teamId, items } = data as { teamId: number; items: TeamMemberResponse[] }
          state.teamMembers[teamId] = items
          break
        }
        case 'addTeamMember': {
          const item = data as TeamMemberResponse & { team_id: number }
          state.teamMembers[item.team_id] = [
            ...(state.teamMembers[item.team_id] ?? []),
            item,
          ]
          break
        }
        case 'removeTeamMember': {
          const { teamId, userId } = data as { teamId: number; userId: number }
          state.teamMembers[teamId] = (state.teamMembers[teamId] ?? []).filter(
            (member) => member.user_id !== userId,
          )
          break
        }
        case 'teamRepos': {
          const { teamId, items } = data as { teamId: number; items: TeamRepoResponse[] }
          state.teamRepos[teamId] = items
          break
        }
        case 'grantRepo': {
          const item = data as TeamRepoResponse
          state.teamRepos[item.team_id] = [
            ...(state.teamRepos[item.team_id] ?? []).filter(
              (grant) => grant.id !== item.id,
            ),
            item,
          ]
          break
        }
        case 'revokeRepo': {
          const { teamId, repoId } = data as { teamId: number; repoId: number }
          state.teamRepos[teamId] = (state.teamRepos[teamId] ?? []).filter(
            (grant) => grant.id !== repoId,
          )
          break
        }
      }
    },
    orgFailed(
      state,
      action: PayloadAction<{ part: OrgPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    organizationsReset() {
      return initialState
    },
  },
})

export const { orgRequested, orgSucceeded, orgFailed, organizationsReset } =
  organizationsSlice.actions

export default organizationsSlice.reducer
