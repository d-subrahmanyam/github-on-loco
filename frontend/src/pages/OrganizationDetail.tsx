import { FaBuilding, FaPlus, FaTrash, FaUsers } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useParams } from 'react-router'
import { orgRequested } from '../store/organizationsSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, secondaryButton } from '../ui'
import type { TeamResponse } from '../bindings/TeamResponse'

const ROLES = ['read', 'write', 'admin'] as const

/** One organization: members, teams, and each team's repository grants. */
export function OrganizationDetail() {
  const dispatch = useAppDispatch()
  const { handle = '' } = useParams()
  const store = useAppSelector((state) => state.organizations)
  const { current, members, teams, teamMembers, teamRepos, loading, error } = store
  const [memberName, setMemberName] = useState('')
  const [memberRole, setMemberRole] = useState('member')
  const [teamName, setTeamName] = useState('')

  useEffect(() => {
    const key = handle
    dispatch(orgRequested({ part: 'show', key: `show:${key}`, handle }))
    dispatch(orgRequested({ part: 'members', key: `members:${key}`, handle }))
    dispatch(orgRequested({ part: 'teams', key: `teams:${key}`, handle }))
  }, [dispatch, handle])

  function useTeam(teamId: number) {
    dispatch(
      orgRequested({
        part: 'teamMembers',
        key: `team-members:${handle}:${teamId}`,
        handle,
        teamId,
      }),
    )
    dispatch(
      orgRequested({
        part: 'teamRepos',
        key: `team-repos:${handle}:${teamId}`,
        handle,
        teamId,
      }),
    )
  }

  useEffect(() => {
    teams.forEach((team) => useTeam(team.id))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dispatch, handle, teams.map((team) => team.id).join(',')])

  const isOwner = current?.viewer_role === 'owner'

  function addMember(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      orgRequested({
        part: 'addMember',
        key: `add-member:${handle}:${Date.now()}`,
        handle,
        params: { username: memberName, role: memberRole },
      }),
    )
    setMemberName('')
    setMemberRole('member')
  }

  function createTeam(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      orgRequested({
        part: 'createTeam',
        key: `create-team:${handle}:${Date.now()}`,
        handle,
        params: { name: teamName, description: null },
      }),
    )
    setTeamName('')
  }

  return (
    <div className="space-y-6">
      <section className={`${card} p-6`}>
        <h1 className="flex items-center gap-2 text-2xl font-semibold">
          <FaBuilding /> {current?.display_name ?? handle}
        </h1>
        <p className="mt-1 text-sm text-slate-500">@{handle} · {current?.viewer_role ?? '…'}</p>
        {current?.description && <p className="mt-3 text-slate-600">{current.description}</p>}
      </section>

      <div className="grid gap-6 lg:grid-cols-2">
        <section className={`${card} overflow-hidden`}>
          <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
            <FaUsers className="text-slate-500" />
            <h2 className="text-lg font-semibold">Members</h2>
          </div>
          <ul className="divide-y divide-slate-100">
            {members.map((member) => (
              <li key={member.id} className="flex items-center justify-between px-5 py-3">
                <span className="text-sm text-slate-800">{member.username}</span>
                <div className="flex items-center gap-3">
                  <span className="text-xs text-slate-500">{member.role}</span>
                  {isOwner && (
                    <button
                      type="button"
                      title={`Remove ${member.username}`}
                      onClick={() =>
                        dispatch(
                          orgRequested({
                            part: 'removeMember',
                            key: `remove-member:${handle}:${member.user_id}:${Date.now()}`,
                            handle,
                            userId: member.user_id,
                          }),
                        )
                      }
                      className="rounded p-1 text-red-600 hover:bg-red-50"
                    >
                      <FaTrash />
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
          {isOwner && (
            <form onSubmit={addMember} className="space-y-3 border-t border-slate-200 p-5">
              <div className="flex gap-2">
                <input
                  required
                  value={memberName}
                  onChange={(event) => setMemberName(event.target.value)}
                  placeholder="username"
                  className={input}
                  aria-label="New member username"
                />
                <select
                  value={memberRole}
                  onChange={(event) => setMemberRole(event.target.value)}
                  className="rounded-lg border border-slate-300 px-2"
                  aria-label="New member role"
                >
                  <option value="member">member</option>
                  <option value="owner">owner</option>
                </select>
              </div>
              <button type="submit" disabled={loading.addMember} className={secondaryButton}>
                <FaPlus /> Add member
              </button>
              {error.addMember && <p role="alert" className="text-sm text-red-700">{error.addMember}</p>}
            </form>
          )}
        </section>

        <section className={`${card} overflow-hidden`}>
          <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
            <FaUsers className="text-slate-500" />
            <h2 className="text-lg font-semibold">Teams</h2>
          </div>
          {teams.length === 0 && <p className="p-5 text-sm text-slate-500">No teams yet.</p>}
          {isOwner && (
            <form onSubmit={createTeam} className="flex gap-2 border-b border-slate-100 p-5">
              <input
                required
                value={teamName}
                onChange={(event) => setTeamName(event.target.value)}
                placeholder="team name"
                className={input}
                aria-label="New team name"
              />
              <button type="submit" disabled={loading.createTeam} className={secondaryButton}>
                <FaPlus /> Add
              </button>
            </form>
          )}
          {error.createTeam && <p role="alert" className="p-5 text-sm text-red-700">{error.createTeam}</p>}
          <ul className="divide-y divide-slate-100">
            {teams.map((team: TeamResponse) => (
              <li key={team.id} className="px-5 py-4">
                <p className="font-medium">{team.name}</p>
                <p className="mt-1 text-xs text-slate-500">{team.member_count} members</p>
                <TeamEditor
                  team={team}
                  isOwner={isOwner}
                  handle={handle}
                  members={teamMembers[team.id] ?? []}
                  repos={teamRepos[team.id] ?? []}
                  loading={loading}
                />
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  )
}

/** Team membership and repository grants. Kept inline to avoid another route. */
function TeamEditor({
  team,
  isOwner,
  handle,
  members,
  repos,
  loading,
}: {
  team: TeamResponse
  isOwner: boolean
  handle: string
  members: { id: number; user_id: number; username: string }[]
  repos: { id: number; team_id: number; owner: string; name: string; role: string }[]
  loading: Record<string, boolean>
}) {
  const dispatch = useAppDispatch()
  const [username, setUsername] = useState('')
  const [repository, setRepository] = useState('')
  const [role, setRole] = useState<string>('write')

  return (
    <div className="mt-4 space-y-4 rounded-lg bg-slate-50 p-3">
      <div>
        <p className="text-xs font-semibold uppercase text-slate-500">Members</p>
        <ul className="mt-1 space-y-1">
          {members.map((member) => (
            <li key={member.id} className="flex items-center justify-between text-sm">
              <span>{member.username}</span>
              {isOwner && (
                <button
                  type="button"
                  title={`Remove ${member.username}`}
                  onClick={() =>
                    dispatch(
                      orgRequested({
                        part: 'removeTeamMember',
                        key: `rm-tm:${handle}:${team.id}:${member.user_id}:${Date.now()}`,
                        handle,
                        teamId: team.id,
                        userId: member.user_id,
                      }),
                    )
                  }
                  className="text-red-600"
                >
                  <FaTrash />
                </button>
              )}
            </li>
          ))}
          {members.length === 0 && <li className="text-sm text-slate-400">No members</li>}
        </ul>
        {isOwner && (
          <form
            className="mt-2 flex gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              dispatch(
                orgRequested({
                  part: 'addTeamMember',
                  key: `add-tm:${handle}:${team.id}:${Date.now()}`,
                  handle,
                  teamId: team.id,
                  params: { username, role: 'member' },
                }),
              )
              setUsername('')
            }}
          >
            <input
              required
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              placeholder="org member"
              className={input}
              aria-label={`Member for ${team.name}`}
            />
            <button type="submit" disabled={loading.addTeamMember} className={secondaryButton}>
              Add
            </button>
          </form>
        )}
      </div>
      <div>
        <p className="text-xs font-semibold uppercase text-slate-500">Repository grants</p>
        <ul className="mt-1 space-y-1">
          {repos.map((grant) => (
            <li key={grant.id} className="flex items-center justify-between text-sm">
              <Link
                to={`/repos/${grant.owner}/${grant.name}`}
                className="text-blue-600 hover:underline"
              >
                {grant.owner}/{grant.name}
              </Link>
              <div className="flex items-center gap-2">
                <span className="text-xs text-slate-500">{grant.role}</span>
                {isOwner && (
                  <button
                    type="button"
                    title="Revoke grant"
                    onClick={() =>
                      dispatch(
                        orgRequested({
                          part: 'revokeRepo',
                          key: `revoke:${handle}:${team.id}:${grant.id}:${Date.now()}`,
                          handle,
                          teamId: team.id,
                          repoId: grant.id,
                        }),
                      )
                    }
                    className="text-red-600"
                  >
                    <FaTrash />
                  </button>
                )}
              </div>
            </li>
          ))}
          {repos.length === 0 && <li className="text-sm text-slate-400">No grants</li>}
        </ul>
        {isOwner && (
          <form
            className="mt-2 flex gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              dispatch(
                orgRequested({
                  part: 'grantRepo',
                  key: `grant:${handle}:${team.id}:${Date.now()}`,
                  handle,
                  teamId: team.id,
                  params: { repository, role },
                }),
              )
              setRepository('')
            }}
          >
            <input
              required
              value={repository}
              onChange={(event) => setRepository(event.target.value)}
              placeholder="repo name"
              className={input}
              aria-label={`Repository for ${team.name}`}
            />
            <select
              value={role}
              onChange={(event) => setRole(event.target.value)}
              className="rounded-lg border border-slate-300 px-2"
              aria-label={`Role for ${team.name}`}
            >
              {ROLES.map((value) => (
                <option key={value} value={value}>{value}</option>
              ))}
            </select>
            <button type="submit" disabled={loading.grantRepo} className={secondaryButton}>
              Grant
            </button>
          </form>
        )}
      </div>
    </div>
  )
}
