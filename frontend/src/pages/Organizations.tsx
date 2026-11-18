import { FaBuilding, FaPlus } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router'
import { orgRequested } from '../store/organizationsSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label, primaryButton } from '../ui'

/** Organization directory and creation form. */
export function Organizations() {
  const dispatch = useAppDispatch()
  const { orgs, loading, error } = useAppSelector((store) => store.organizations)
  const [handle, setHandle] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [description, setDescription] = useState('')

  useEffect(() => {
    dispatch(orgRequested({ part: 'list', key: 'list:orgs' }))
  }, [dispatch])

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      orgRequested({
        part: 'create',
        key: `create:${handle}:${Date.now()}`,
        params: { handle, display_name: displayName, description: description || null },
      }),
    )
    setHandle('')
    setDisplayName('')
    setDescription('')
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_340px]">
      <section className={`${card} overflow-hidden`}>
        <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
          <FaBuilding className="text-slate-500" />
          <h2 className="text-lg font-semibold">Your organizations</h2>
        </div>
        {loading.list && <p className="p-5 text-sm text-slate-500">Loading…</p>}
        {error.list && <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>}
        {!loading.list && orgs.length === 0 && (
          <p className="p-5 text-sm text-slate-500">You are not a member of any organization yet.</p>
        )}
        <ul className="divide-y divide-slate-100">
          {orgs.map((org) => (
            <li key={org.id} className="px-5 py-4">
              <Link
                to={`/organizations/${org.handle}`}
                className="text-base font-medium text-blue-600 hover:underline"
              >
                {org.display_name}
              </Link>
              <p className="mt-1 text-xs text-slate-500">
                @{org.handle} · {org.viewer_role}
              </p>
              {org.description && <p className="mt-1 text-sm text-slate-600">{org.description}</p>}
            </li>
          ))}
        </ul>
      </section>

      <aside>
        <form onSubmit={handleSubmit} className={`${card} h-fit space-y-4 p-5`}>
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaPlus className="text-blue-600" /> New organization
          </h2>
          <div>
            <label htmlFor="org-handle" className={label}>Handle</label>
            <input
              id="org-handle"
              required
              value={handle}
              onChange={(event) => setHandle(event.target.value)}
              placeholder="acme"
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="org-name" className={label}>Display name</label>
            <input
              id="org-name"
              required
              value={displayName}
              onChange={(event) => setDisplayName(event.target.value)}
              placeholder="Acme Inc."
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="org-description" className={label}>Description</label>
            <input
              id="org-description"
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              className={`${input} mt-1`}
            />
          </div>
          <button type="submit" disabled={loading.create} className={`${primaryButton} w-full`}>
            <FaPlus /> {loading.create ? 'Creating…' : 'Create organization'}
          </button>
          {error.create && <p role="alert" className="text-sm text-red-700">{error.create}</p>}
        </form>
      </aside>
    </div>
  )
}
