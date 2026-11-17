import { FaExclamationCircle, FaPlus, FaTag } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useSearchParams } from 'react-router'
import { issueRequested } from '../store/issuesSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label, primaryButton } from '../ui'
import { useRepoParams } from './RepoLayout'

/** Issue list, state filter and new-issue form. */
export function IssueList() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const state = searchParams.get('state') ?? ''
  const { issues, labels, loading, error } = useAppSelector((state) => state.issues)
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [selected, setSelected] = useState<number[]>([])
  const [formError, setFormError] = useState<string | null>(null)

  const scope = `${owner}/${name}`

  useEffect(() => {
    dispatch(
      issueRequested({
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
      issueRequested({ part: 'labels', key: `labels:${scope}`, owner, name }),
    )
  }, [dispatch, owner, name, scope])

  function setStateFilter(next: string) {
    const params = new URLSearchParams(searchParams)
    if (next) params.set('state', next)
    else params.delete('state')
    setSearchParams(params)
  }

  function toggleLabel(id: number) {
    setSelected((current) =>
      current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
    )
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setFormError(null)
    dispatch(
      issueRequested({
        part: 'create',
        key: `create:${scope}:${Date.now()}`,
        owner,
        name,
        listState: state || undefined,
        params: {
          title,
          body: body || null,
          label_ids: selected,
        },
      }),
    )
    setTitle('')
    setBody('')
    setSelected([])
  }

  const filterClass = (active: boolean) =>
    `rounded-lg px-3 py-1 text-sm ${
      active ? 'bg-slate-900 text-white' : 'border border-slate-300 text-slate-600 hover:bg-slate-50'
    }`

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
      <section className={`${card} overflow-hidden`}>
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 bg-slate-50 px-5 py-4">
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaExclamationCircle className="text-slate-500" /> Issues
          </h2>
          <div className="flex gap-2">
            <button type="button" className={filterClass(state === '')} onClick={() => setStateFilter('')}>
              All
            </button>
            <button type="button" className={filterClass(state === 'open')} onClick={() => setStateFilter('open')}>
              Open
            </button>
            <button type="button" className={filterClass(state === 'closed')} onClick={() => setStateFilter('closed')}>
              Closed
            </button>
          </div>
        </div>
        {loading.list && <p className="p-5 text-sm text-slate-500">Loading issues…</p>}
        {error.list && <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>}
        <ul className="divide-y divide-slate-100">
          {issues.length === 0 && !loading.list && (
            <li className="p-5 text-sm text-slate-500">No issues yet. Open the first one.</li>
          )}
          {issues.map((issue) => (
            <li key={issue.id} className="px-5 py-4 hover:bg-slate-50">
              <div className="flex items-start gap-3">
                <span
                  className={`mt-1 inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                    issue.state === 'open'
                      ? 'bg-green-100 text-green-800'
                      : 'bg-slate-200 text-slate-700'
                  }`}
                >
                  {issue.state}
                </span>
                <div className="flex-1">
                  <Link
                    to={`${issue.number}${state ? `?state=${encodeURIComponent(state)}` : ''}`}
                    className="font-medium text-blue-600 hover:underline"
                  >
                    {issue.title}
                  </Link>
                  <p className="mt-1 text-xs text-slate-500">
                    #{issue.number} opened by {issue.author} · {issue.comment_count} comments
                  </p>
                  {issue.labels.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-1">
                      {issue.labels.map((item) => (
                        <span
                          key={item.id}
                          className="inline-flex items-center gap-1 rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-700"
                        >
                          <span
                            className="inline-block h-2 w-2 rounded-full"
                            style={{ backgroundColor: `#${item.color}` }}
                          />
                          {item.name}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </li>
          ))}
        </ul>
      </section>

      <aside className="space-y-6">
        <form onSubmit={handleSubmit} className={`${card} h-fit space-y-4 p-5`}>
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaPlus className="text-blue-600" /> New issue
          </h2>
          <div>
            <label htmlFor="issue-title" className={label}>Title</label>
            <input
              id="issue-title"
              required
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              className={`${input} mt-1`}
              placeholder="Something is broken"
            />
          </div>
          <div>
            <label htmlFor="issue-body" className={label}>Description</label>
            <textarea
              id="issue-body"
              rows={4}
              value={body}
              onChange={(event) => setBody(event.target.value)}
              className={`${input} mt-1`}
            />
          </div>
          {labels.length > 0 && (
            <div>
              <span className={label}>Labels</span>
              <div className="mt-2 space-y-1">
                {labels.map((item) => (
                  <label key={item.id} className="flex items-center gap-2 text-sm text-slate-700">
                    <input
                      type="checkbox"
                      checked={selected.includes(item.id)}
                      onChange={() => toggleLabel(item.id)}
                    />
                    <FaTag style={{ color: `#${item.color}` }} />
                    {item.name}
                  </label>
                ))}
              </div>
            </div>
          )}
          <button type="submit" disabled={loading.create} className={`${primaryButton} w-full`}>
            <FaPlus /> {loading.create ? 'Creating…' : 'Create issue'}
          </button>
          {(formError || error.create) && (
            <p role="alert" className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">
              {formError || error.create}
            </p>
          )}
        </form>
      </aside>
    </div>
  )
}
