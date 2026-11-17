import { FaPlus, FaTrash, FaUsers } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { collaboratorRequested } from '../store/collaboratorsSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label, primaryButton } from '../ui'
import { useRepoParams } from './RepoLayout'

const ROLES = ['read', 'write', 'admin'] as const

/** Manage who can access this repository besides its owner. */
export function Collaborators() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const { collaborators, loading, error } = useAppSelector((store) => store.collaborators)
  const [username, setUsername] = useState('')
  const [role, setRole] = useState<string>('read')

  const scope = `${owner}/${name}`

  useEffect(() => {
    dispatch(
      collaboratorRequested({
        part: 'list',
        key: `list:${scope}`,
        owner,
        name,
      }),
    )
  }, [dispatch, owner, name, scope])

  function handleAdd(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      collaboratorRequested({
        part: 'add',
        key: `add:${scope}:${Date.now()}`,
        owner,
        name,
        username,
        role,
      }),
    )
    setUsername('')
    setRole('read')
  }

  function changeRole(userId: number, next: string) {
    dispatch(
      collaboratorRequested({
        part: 'update',
        key: `update:${scope}:${userId}:${Date.now()}`,
        owner,
        name,
        userId,
        role: next,
      }),
    )
  }

  function remove(userId: number) {
    dispatch(
      collaboratorRequested({
        part: 'remove',
        key: `remove:${scope}:${userId}:${Date.now()}`,
        owner,
        name,
        userId,
      }),
    )
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
      <section className={`${card} overflow-hidden`}>
        <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
          <FaUsers className="text-slate-500" />
          <h2 className="text-lg font-semibold">Collaborators</h2>
        </div>
        {loading.list && <p className="p-5 text-sm text-slate-500">Loading collaborators…</p>}
        {error.list && <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>}
        <ul className="divide-y divide-slate-100">
          {collaborators.length === 0 && !loading.list && (
            <li className="p-5 text-sm text-slate-500">No collaborators yet.</li>
          )}
          {collaborators.map((collaborator) => (
            <li key={collaborator.id} className="flex flex-wrap items-center gap-3 px-5 py-4">
              <div className="flex-1">
                <p className="font-medium text-slate-800">{collaborator.username}</p>
                <p className="text-xs text-slate-500">#{collaborator.user_id}</p>
              </div>
              <select
                aria-label={`Role for ${collaborator.username}`}
                value={collaborator.role}
                onChange={(event) => changeRole(collaborator.user_id, event.target.value)}
                className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm"
              >
                {ROLES.map((value) => (
                  <option key={value} value={value}>{value}</option>
                ))}
              </select>
              <button
                type="button"
                title={`Remove ${collaborator.username}`}
                onClick={() => remove(collaborator.user_id)}
                className="rounded-lg p-2 text-red-600 hover:bg-red-50"
              >
                <FaTrash />
              </button>
            </li>
          ))}
        </ul>
        {error.update && <p role="alert" className="p-5 text-sm text-red-700">{error.update}</p>}
        {error.remove && <p role="alert" className="p-5 text-sm text-red-700">{error.remove}</p>}
      </section>

      <aside>
        <form onSubmit={handleAdd} className={`${card} h-fit space-y-4 p-5`}>
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaPlus className="text-blue-600" /> Add collaborator
          </h2>
          <div>
            <label htmlFor="collab-username" className={label}>Username</label>
            <input
              id="collab-username"
              required
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              placeholder="their-public-handle"
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="collab-role" className={label}>Role</label>
            <select
              id="collab-role"
              value={role}
              onChange={(event) => setRole(event.target.value)}
              className={`${input} mt-1`}
            >
              {ROLES.map((value) => (
                <option key={value} value={value}>{value}</option>
              ))}
            </select>
            <p className="mt-2 text-xs text-slate-500">
              read = browse/clone · write = also push and open PRs · admin = also manage access
            </p>
          </div>
          <button type="submit" disabled={loading.add} className={`${primaryButton} w-full`}>
            <FaPlus /> {loading.add ? 'Adding…' : 'Add collaborator'}
          </button>
          {error.add && <p role="alert" className="text-sm text-red-700">{error.add}</p>}
        </form>
      </aside>
    </div>
  )
}
