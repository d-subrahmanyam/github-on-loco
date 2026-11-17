import { useEffect } from 'react'
import { useSearchParams } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
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

  return (
    <div>
      <div>
        <label htmlFor="branch">Branch</label>{' '}
        <select
          id="branch"
          value={ref ?? ''}
          onChange={(event) => update({ ref: event.target.value, path: '' })}
        >
          <option value="">default</option>
          {branches.map((branch) => (
            <option key={branch.name} value={branch.name}>
              {branch.name}
            </option>
          ))}
        </select>
      </div>

      <div>
        <button type="button" onClick={() => update({ path: '' })}>
          {name}
        </button>
        {segments.map((segment, index) => (
          <span key={`${segment}-${index}`}>
            {' / '}
            <button
              type="button"
              onClick={() => update({ path: segments.slice(0, index + 1).join('/') })}
            >
              {segment}
            </button>
          </span>
        ))}
      </div>

      {loading.tree && <p>Loading tree…</p>}
      {error.tree && <p role="alert">{error.tree}</p>}
      {!path && (
        <ul>
          {tree.map((entry) => (
            <li key={entry.sha + entry.path}>
              <button type="button" onClick={() => update({ path: entry.path })}>
                {entry.kind === 'tree' ? '📁' : '📄'} {entry.name}
              </button>
            </li>
          ))}
        </ul>
      )}

      {path && (
        <div>
          <button type="button" onClick={() => update({ path: '' })}>
            ← Back to directory
          </button>
          {loading.file && <p>Loading file…</p>}
          {error.file && <p role="alert">{error.file}</p>}
          {file !== null && <pre>{file}</pre>}
        </div>
      )}
    </div>
  )
}
