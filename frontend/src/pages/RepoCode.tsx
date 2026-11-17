import { FaArrowLeft, FaCodeBranch, FaFile, FaFolder } from 'react-icons/fa'
import { useEffect } from 'react'
import { useSearchParams } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label } from '../ui'
import { useRepoParams } from './RepoLayout'

/** Tree browser: pick a branch, walk directories and read file contents. */
export function RepoCode() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const ref = searchParams.get('ref') ?? undefined
  const path = searchParams.get('path') ?? ''
  const { branches, tree, file, loading, error } = useAppSelector(
    (state) => state.browse,
  )

  useEffect(() => {
    dispatch(
      browseRequested({ part: 'branches', key: `${owner}/${name}`, owner, name }),
    )
  }, [dispatch, owner, name])

  useEffect(() => {
    dispatch(
      browseRequested({
        part: 'tree',
        key: `${owner}/${name}@${ref ?? 'default'}:${path}`,
        owner,
        name,
        ref,
        path,
      }),
    )
  }, [dispatch, owner, name, ref, path])

  useEffect(() => {
    if (!path) return
    dispatch(
      browseRequested({
        part: 'file',
        key: `${owner}/${name}@${ref ?? 'default'}:${path}`,
        owner,
        name,
        ref,
        path,
      }),
    )
  }, [dispatch, owner, name, ref, path])

  function update(next: { ref?: string | undefined; path?: string }) {
    const params = new URLSearchParams(searchParams)
    if (next.ref !== undefined) {
      if (next.ref) params.set('ref', next.ref)
      else params.delete('ref')
    }
    if (next.path !== undefined) {
      if (next.path) params.set('path', next.path)
      else params.delete('path')
    }
    setSearchParams(params)
  }

  const segments = path.split('/').filter(Boolean)
  const crumbClass = 'text-blue-600 hover:underline'

  return (
    <section className={`${card} overflow-hidden`}>
      <div className="flex flex-wrap items-center gap-4 border-b border-slate-200 bg-slate-50 px-5 py-3">
        <div className="flex items-center gap-2">
          <FaCodeBranch className="text-slate-500" />
          <label htmlFor="branch" className={`${label} sr-only`}>
            Branch
          </label>
          <select
            id="branch"
            value={ref ?? ''}
            onChange={(event) => update({ ref: event.target.value, path: '' })}
            className={`${input} w-auto py-1`}
          >
            <option value="">default branch</option>
            {branches.map((branch) => (
              <option key={branch.name} value={branch.name}>
                {branch.name}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-wrap items-center gap-1 text-sm">
          <button type="button" className={crumbClass} onClick={() => update({ path: '' })}>
            {name}
          </button>
          {segments.map((segment, index) => (
            <span key={`${segment}-${index}`}>
              <span className="mx-1 text-slate-400">/</span>
              <button
                type="button"
                className={crumbClass}
                onClick={() => update({ path: segments.slice(0, index + 1).join('/') })}
              >
                {segment}
              </button>
            </span>
          ))}
        </div>
      </div>

      {loading.tree && <p className="p-5 text-sm text-slate-500">Loading tree…</p>}
      {error.tree && <p role="alert" className="p-5 text-sm text-red-700">{error.tree}</p>}

      {!path && (
        <ul className="divide-y divide-slate-100">
          {tree.length === 0 && !loading.tree && (
            <li className="p-5 text-sm text-slate-500">This directory is empty.</li>
          )}
          {tree.map((entry) => (
            <li key={entry.sha + entry.path}>
              <button
                type="button"
                onClick={() => update({ path: entry.path })}
                className="flex w-full items-center gap-3 px-5 py-3 text-left text-sm hover:bg-slate-50"
              >
                {entry.kind === 'tree' ? (
                  <FaFolder className="text-blue-500" />
                ) : (
                  <FaFile className="text-slate-400" />
                )}
                <span className="font-medium text-slate-800">{entry.name}</span>
                <span className="ml-auto font-mono text-xs text-slate-400">
                  {entry.sha.slice(0, 8)}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}

      {path && (
        <div className="p-5">
          <button
            type="button"
            onClick={() => update({ path: '' })}
            className="mb-4 inline-flex items-center gap-2 text-sm text-blue-600 hover:underline"
          >
            <FaArrowLeft /> Back to directory
          </button>
          {loading.file && <p className="text-sm text-slate-500">Loading file…</p>}
          {error.file && <p role="alert" className="text-sm text-red-700">{error.file}</p>}
          {file !== null && (
            <pre className="overflow-x-auto rounded-lg border border-slate-200 bg-slate-950 p-4 text-sm leading-relaxed text-slate-100">
              {file}
            </pre>
          )}
        </div>
      )}
    </section>
  )
}
