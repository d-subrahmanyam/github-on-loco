import { FaCodeBranch, FaPlus } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { pullRequested } from '../store/pullsSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label, primaryButton } from '../ui'
import { canWriteRole, useRepoParams, useRepoRole } from './RepoLayout'

/** Pull request list, state filter and new-PR form. */
export function PullList() {
  const dispatch = useAppDispatch()
  const navigate = useNavigate()
  const { owner, name } = useRepoParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const state = searchParams.get('state') ?? ''
  const { pulls, current, loading, error } = useAppSelector((store) => store.pulls)
  const branches = useAppSelector((store) => store.browse.branches)
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [base, setBase] = useState('')
  const [head, setHead] = useState('')
  const [pendingNavigate, setPendingNavigate] = useState(false)

  const scope = `${owner}/${name}`
  const canWrite = canWriteRole(useRepoRole())

  useEffect(() => {
    dispatch(
      pullRequested({
        part: 'list',
        key: `list:${scope}:${state}`,
        owner,
        name,
        state: state || undefined,
        listState: state || undefined,
      }),
    )
  }, [dispatch, owner, name, scope, state])

  useEffect(() => {
    dispatch(
      browseRequested({ part: 'branches', key: `branches:${scope}`, owner, name }),
    )
  }, [dispatch, owner, name, scope])

  useEffect(() => {
    if (branches.length > 0) {
      if (!base) setBase(branches[0].name)
      if (!head) setHead(branches[1]?.name ?? branches[0].name)
    }
  }, [branches, base, head])

  // After a successful create, jump to the new PR's detail page.
  useEffect(() => {
    if (pendingNavigate && !loading.create && !error.create && current) {
      setPendingNavigate(false)
      navigate(`${current.number}`)
    }
  }, [pendingNavigate, loading.create, error.create, current, navigate])

  function setStateFilter(next: string) {
    const params = new URLSearchParams(searchParams)
    if (next) params.set('state', next)
    else params.delete('state')
    setSearchParams(params)
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setPendingNavigate(true)
    dispatch(
      pullRequested({
        part: 'create',
        key: `create:${scope}:${Date.now()}`,
        owner,
        name,
        listState: state || undefined,
        params: { title, body: body || null, head_ref: head, base_ref: base },
      }),
    )
    setTitle('')
    setBody('')
  }

  const filterClass = (active: boolean) =>
    `rounded-lg px-3 py-1 text-sm ${
      active ? 'bg-slate-900 text-white' : 'border border-slate-300 text-slate-600 hover:bg-slate-50'
    }`

  const badgeClass = (value: string) =>
    value === 'open'
      ? 'bg-green-100 text-green-800'
      : value === 'merged'
        ? 'bg-purple-100 text-purple-800'
        : 'bg-slate-200 text-slate-700'

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
      <section className={`${card} overflow-hidden`}>
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 bg-slate-50 px-5 py-4">
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaCodeBranch className="text-slate-500" /> Pull requests
          </h2>
          <div className="flex gap-2">
            {[
              ['', 'All'],
              ['open', 'Open'],
              ['closed', 'Closed'],
              ['merged', 'Merged'],
            ].map(([value, text]) => (
              <button
                key={value}
                type="button"
                className={filterClass(state === value)}
                onClick={() => setStateFilter(value)}
              >
                {text}
              </button>
            ))}
          </div>
        </div>
        {loading.list && <p className="p-5 text-sm text-slate-500">Loading pull requests…</p>}
        {error.list && <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>}
        <ul className="divide-y divide-slate-100">
          {pulls.length === 0 && !loading.list && (
            <li className="p-5 text-sm text-slate-500">
              No pull requests yet. Open the first one.
            </li>
          )}
          {pulls.map((pr) => (
            <li key={pr.id} className="px-5 py-4 hover:bg-slate-50">
              <div className="flex items-start gap-3">
                <span className={`mt-1 rounded-full px-2 py-0.5 text-xs font-medium ${badgeClass(pr.state)}`}>
                  {pr.state}
                </span>
                <div className="flex-1">
                  <Link
                    to={`${pr.number}${state ? `?state=${encodeURIComponent(state)}` : ''}`}
                    className="font-medium text-blue-600 hover:underline"
                  >
                    {pr.title}
                  </Link>
                  <p className="mt-1 text-xs text-slate-500">
                    #{pr.number} opened by {pr.author} · {pr.comment_count} comments
                  </p>
                  <p className="mt-1 font-mono text-xs text-slate-500">
                    {pr.head_ref} <span className="text-slate-400">→</span> {pr.base_ref}
                  </p>
                </div>
              </div>
            </li>
          ))}
        </ul>
      </section>

      {canWrite && (
      <aside className="space-y-6">
        <form onSubmit={handleSubmit} className={`${card} h-fit space-y-4 p-5`}>
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaPlus className="text-blue-600" /> New pull request
          </h2>
          <div>
            <label htmlFor="pr-title" className={label}>Title</label>
            <input
              id="pr-title"
              required
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              className={`${input} mt-1`}
              placeholder="Describe your change"
            />
          </div>
          <div>
            <label htmlFor="pr-body" className={label}>Description</label>
            <textarea
              id="pr-body"
              rows={4}
              value={body}
              onChange={(event) => setBody(event.target.value)}
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="pr-base" className={label}>Base branch</label>
            <select
              id="pr-base"
              required
              value={base}
              onChange={(event) => setBase(event.target.value)}
              className={`${input} mt-1`}
            >
              {branches.map((branch) => (
                <option key={branch.name} value={branch.name}>{branch.name}</option>
              ))}
            </select>
          </div>
          <div>
            <label htmlFor="pr-head" className={label}>Compare branch</label>
            <select
              id="pr-head"
              required
              value={head}
              onChange={(event) => setHead(event.target.value)}
              className={`${input} mt-1`}
            >
              {branches.map((branch) => (
                <option key={branch.name} value={branch.name}>{branch.name}</option>
              ))}
            </select>
          </div>
          <button type="submit" disabled={loading.create} className={`${primaryButton} w-full`}>
            <FaPlus /> {loading.create ? 'Creating…' : 'Create pull request'}
          </button>
          {error.create && (
            <p role="alert" className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">
              {error.create}
            </p>
          )}
        </form>
      </aside>
      )}
    </div>
  )
}
