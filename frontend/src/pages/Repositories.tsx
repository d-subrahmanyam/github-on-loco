import { FaLink, FaLock, FaPlus, FaTimes, FaUnlock } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { orgRequested } from '../store/organizationsSlice'
import {
  clearCreateError,
  createRequested,
  listRequested,
} from '../store/repositoriesSlice'
import { card, codeBox, input, label, primaryButton } from '../ui'

/** Dashboard: list the current user's repositories and create new ones. */
export function Repositories() {
  const dispatch = useAppDispatch()
  const { items, loading, error, creating, createError } = useAppSelector(
    (state) => state.repositories,
  )
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [isPrivate, setIsPrivate] = useState(false)
  const [orgHandle, setOrgHandle] = useState('')
  const orgs = useAppSelector((state) => state.organizations.orgs)

  useEffect(() => {
    dispatch(listRequested())
    dispatch(orgRequested({ part: 'list', key: 'list:orgs' }))
  }, [dispatch])

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      createRequested({
        name,
        description: description || null,
        is_private: isPrivate,
        organization: orgHandle || null,
      }),
    )
    setName('')
    setDescription('')
    setIsPrivate(false)
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <img
          src="/github-loco-logo.png"
          alt="github-on-loco logo"
          className="h-9 w-auto"
        />
        <div>
          <h1 className="text-2xl font-semibold">Your repositories</h1>
          <p className="text-sm text-slate-600">Create a repo and start pushing code.</p>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-[320px_1fr]">
        <section className={`${card} h-fit p-6`}>
          <h2 className="mb-4 flex items-center gap-2 text-lg font-semibold">
            <FaPlus className="text-blue-600" /> New repository
          </h2>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="repo-name" className={label}>
                Name
              </label>
              <input
                id="repo-name"
                required
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="hello-world"
                className={`${input} mt-1`}
              />
            </div>
            <div>
              <label htmlFor="repo-description" className={label}>
                Description
              </label>
              <input
                id="repo-description"
                value={description}
                onChange={(event) => setDescription(event.target.value)}
                placeholder="What is this repo about?"
                className={`${input} mt-1`}
              />
            </div>
            {orgs.length > 0 && (
              <div>
                <label htmlFor="repo-owner" className={label}>
                  Owner
                </label>
                <select
                  id="repo-owner"
                  value={orgHandle}
                  onChange={(event) => setOrgHandle(event.target.value)}
                  className={`${input} mt-1`}
                >
                  <option value="">Personal</option>
                  {orgs
                    .filter((org) => org.viewer_role === 'owner')
                    .map((org) => (
                      <option key={org.id} value={org.handle}>
                        {org.display_name} (@{org.handle})
                      </option>
                    ))}
                </select>
              </div>
            )}
            <label htmlFor="repo-private" className="flex items-center gap-2 text-sm text-slate-700">
              <input
                id="repo-private"
                type="checkbox"
                checked={isPrivate}
                onChange={(event) => setIsPrivate(event.target.checked)}
                className="h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-500"
              />
              {isPrivate ? <FaLock className="text-amber-600" /> : <FaUnlock className="text-slate-400" />}
              Private
            </label>
            <button type="submit" disabled={creating} className={`${primaryButton} w-full`}>
              <FaPlus />
              {creating ? 'Creating…' : 'Create repository'}
            </button>
            {createError && (
              <div className="flex items-start justify-between gap-2 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">
                <span>{createError}</span>
                <button
                  type="button"
                  onClick={() => dispatch(clearCreateError())}
                  aria-label="Dismiss error"
                >
                  <FaTimes />
                </button>
              </div>
            )}
          </form>
        </section>

        <section className={`${card} p-6`}>
          <h2 className="mb-4 text-lg font-semibold">Repositories</h2>
          {loading && <p className="text-sm text-slate-500">Loading repositories…</p>}
          {error && <p role="alert" className="text-sm text-red-700">{error}</p>}
          {!loading && items.length === 0 && (
            <p className="text-sm text-slate-500">
              No repositories yet. Create your first one from the form on the left.
            </p>
          )}
          <ul className="divide-y divide-slate-200">
            {items.map((repo) => (
              <li key={repo.id} className="py-4 first:pt-0 last:pb-0">
                <div className="flex flex-wrap items-center gap-2">
                  <Link
                    to={`/repos/${repo.owner}/${repo.name}`}
                    className="font-semibold text-blue-600 hover:underline"
                  >
                    {repo.owner}/{repo.name}
                  </Link>
                  {repo.is_private ? (
                    <span className="inline-flex items-center gap-1 rounded-full bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-800">
                      <FaLock /> Private
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1 rounded-full bg-slate-100 px-2 py-0.5 text-xs font-medium text-slate-600">
                      <FaUnlock /> Public
                    </span>
                  )}
                </div>
                {repo.description && (
                  <p className="mt-1 text-sm text-slate-600">{repo.description}</p>
                )}
                <div className={`${codeBox} mt-2 flex items-center gap-2`}>
                  <FaLink />
                  <span className="truncate">{repo.clone_url}</span>
                </div>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  )
}
